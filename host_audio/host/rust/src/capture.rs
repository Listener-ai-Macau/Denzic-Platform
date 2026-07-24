//! Product-independent host microphone capture over cpal.
//!
//! The platform owns device selection, sample-format dispatch, mono downmix,
//! the capture thread lifecycle, and startup/stop error classification.
//! Products own what happens to the mono f32 frames (resampling, buffering,
//! WAV archival, ASR feeding) via the sink callback, plus their own session
//! and error-wording policy on top of [`CaptureError`].
//!
//! The sink receives mono f32 samples at the *device* sample rate (passed as
//! the second argument, constant per session); converting to the contract
//! rate is a product decision because the two reference products use
//! different resamplers.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{SampleFormat, StreamConfig};
use thiserror::Error;

const STARTUP_TIMEOUT: Duration = Duration::from_secs(5);
const STOP_POLL_INTERVAL: Duration = Duration::from_millis(25);

#[derive(Debug, Clone, Error)]
pub enum CaptureError {
    #[error("no default input microphone")]
    NoDefaultInputDevice,
    #[error("input_devices: {0}")]
    ListInputDevices(String),
    #[error("read default input config: {0}")]
    DefaultInputConfig(String),
    #[error("build input stream: {0}")]
    BuildInputStream(String),
    #[error("start input stream: {0}")]
    StartInputStream(String),
    #[error("unsupported input sample format: {0}")]
    UnsupportedSampleFormat(String),
    #[error("spawn host recorder thread: {0}")]
    SpawnThread(String),
    #[error("host recorder startup timeout: {0}")]
    StartupTimeout(String),
    #[error("host recorder thread panicked")]
    ThreadPanicked,
    #[error("host recorder thread already joined")]
    AlreadyJoined,
}

#[derive(Clone, Debug)]
pub struct InputDeviceInfo {
    pub name: String,
    pub is_default: bool,
}

#[derive(Default)]
pub struct CaptureOptions {
    /// Preferred input device name; `None` selects the system default.
    pub device_name: Option<String>,
    /// Called from the audio thread when cpal reports a runtime stream error.
    pub on_stream_error: Option<Arc<dyn Fn(String) + Send + Sync>>,
}

/// Running capture session. The cpal stream lives on a dedicated thread
/// (cpal streams are `!Send`); `stop` signals the thread and joins it.
pub struct CaptureSession {
    stop_flag: Arc<AtomicBool>,
    join_handle: Option<JoinHandle<Result<(), CaptureError>>>,
}

impl CaptureSession {
    /// Start capturing. `sink` is invoked from the audio callback with mono
    /// f32 frames and the device sample rate. Startup errors (no device,
    /// unsupported format, stream build/play failure) are returned
    /// synchronously via the startup handshake.
    pub fn start<F>(options: CaptureOptions, sink: F) -> Result<Self, CaptureError>
    where
        F: Fn(&[f32], u32) + Send + Sync + 'static,
    {
        let stop_flag = Arc::new(AtomicBool::new(false));
        let stop_for_thread = Arc::clone(&stop_flag);
        let (startup_tx, startup_rx) = mpsc::channel::<Result<(), CaptureError>>();

        let join_handle = thread::Builder::new()
            .name("denzic-host-audio-capture".to_string())
            .spawn(move || run_capture_thread(options, Arc::new(sink), stop_for_thread, startup_tx))
            .map_err(|err| CaptureError::SpawnThread(err.to_string()))?;

        startup_rx
            .recv_timeout(STARTUP_TIMEOUT)
            .map_err(|err| CaptureError::StartupTimeout(err.to_string()))??;

        Ok(Self {
            stop_flag,
            join_handle: Some(join_handle),
        })
    }

    pub fn stop(mut self) -> Result<(), CaptureError> {
        self.stop_flag.store(true, Ordering::SeqCst);
        self.join_handle
            .take()
            .ok_or(CaptureError::AlreadyJoined)?
            .join()
            .map_err(|_| CaptureError::ThreadPanicked)??;
        Ok(())
    }
}

pub fn list_input_devices() -> Result<Vec<InputDeviceInfo>, CaptureError> {
    let host = cpal::default_host();
    let default_name = host
        .default_input_device()
        .and_then(|device| device.name().ok());
    let devices = host
        .input_devices()
        .map_err(|err| CaptureError::ListInputDevices(err.to_string()))?;
    let mut result = Vec::new();
    for device in devices {
        match device.name() {
            Ok(name) => result.push(InputDeviceInfo {
                is_default: default_name.as_deref() == Some(name.as_str()),
                name,
            }),
            Err(_) => continue,
        }
    }
    Ok(result)
}

/// Arithmetic-mean downmix of interleaved samples to mono.
pub fn downmix_to_mono(interleaved: &[f32], channels: usize) -> Vec<f32> {
    let channels = channels.max(1);
    if channels == 1 {
        return interleaved.to_vec();
    }
    interleaved
        .chunks(channels)
        .map(|frame| frame.iter().sum::<f32>() / frame.len() as f32)
        .collect()
}

fn run_capture_thread(
    options: CaptureOptions,
    sink: Arc<dyn Fn(&[f32], u32) + Send + Sync>,
    stop_flag: Arc<AtomicBool>,
    startup_tx: mpsc::Sender<Result<(), CaptureError>>,
) -> Result<(), CaptureError> {
    let host = cpal::default_host();
    let device = match select_input_device(&host, options.device_name.as_deref()) {
        Ok(device) => device,
        Err(err) => {
            let _ = startup_tx.send(Err(err.clone()));
            return Err(err);
        }
    };
    let supported_config = match device.default_input_config() {
        Ok(config) => config,
        Err(err) => {
            let err = CaptureError::DefaultInputConfig(err.to_string());
            let _ = startup_tx.send(Err(err.clone()));
            return Err(err);
        }
    };

    let stream_config: StreamConfig = supported_config.clone().into();
    let sample_rate = stream_config.sample_rate.0;
    let channels = stream_config.channels as usize;
    let on_stream_error = options.on_stream_error.clone();
    let err_fn = move |err: cpal::StreamError| {
        if let Some(handler) = &on_stream_error {
            handler(err.to_string());
        }
    };

    let stream_result = match supported_config.sample_format() {
        SampleFormat::F32 => build_stream::<f32>(
            &device,
            &stream_config,
            channels,
            sample_rate,
            &sink,
            err_fn,
        ),
        SampleFormat::I16 => build_stream::<i16>(
            &device,
            &stream_config,
            channels,
            sample_rate,
            &sink,
            err_fn,
        ),
        SampleFormat::U16 => build_stream::<u16>(
            &device,
            &stream_config,
            channels,
            sample_rate,
            &sink,
            err_fn,
        ),
        other => Err(CaptureError::UnsupportedSampleFormat(format!("{other:?}"))),
    };

    let stream = match stream_result {
        Ok(stream) => stream,
        Err(err) => {
            let _ = startup_tx.send(Err(err.clone()));
            return Err(err);
        }
    };

    if let Err(err) = stream.play() {
        let err = CaptureError::StartInputStream(err.to_string());
        let _ = startup_tx.send(Err(err.clone()));
        return Err(err);
    }
    let _ = startup_tx.send(Ok(()));

    while !stop_flag.load(Ordering::SeqCst) {
        thread::sleep(STOP_POLL_INTERVAL);
    }
    drop(stream);
    Ok(())
}

fn select_input_device(
    host: &cpal::Host,
    device_name: Option<&str>,
) -> Result<cpal::Device, CaptureError> {
    let preferred = device_name.map(str::trim).filter(|name| !name.is_empty());
    if let Some(preferred) = preferred {
        let devices = host
            .input_devices()
            .map_err(|err| CaptureError::ListInputDevices(err.to_string()))?;
        for device in devices {
            if device.name().ok().as_deref() == Some(preferred) {
                return Ok(device);
            }
        }
    }
    host.default_input_device()
        .ok_or(CaptureError::NoDefaultInputDevice)
}

fn build_stream<T>(
    device: &cpal::Device,
    config: &StreamConfig,
    channels: usize,
    sample_rate: u32,
    sink: &Arc<dyn Fn(&[f32], u32) + Send + Sync>,
    err_fn: impl Fn(cpal::StreamError) + Send + 'static,
) -> Result<cpal::Stream, CaptureError>
where
    T: cpal::SizedSample + SampleToF32 + Send + 'static,
{
    let sink = Arc::clone(sink);
    device
        .build_input_stream(
            config,
            move |data: &[T], _| {
                let mono = downmix_to_mono(&samples_to_f32(data), channels);
                sink(&mono, sample_rate);
            },
            err_fn,
            None,
        )
        .map_err(|err| CaptureError::BuildInputStream(err.to_string()))
}

fn samples_to_f32<T: SampleToF32>(data: &[T]) -> Vec<f32> {
    data.iter().map(|sample| sample.to_f32_sample()).collect()
}

/// Sample-format conversion into the [-1.0, 1.0] f32 domain. f32 inputs are
/// clamped; integer formats scale by their positive full-scale value.
pub trait SampleToF32: Copy {
    fn to_f32_sample(self) -> f32;
}

impl SampleToF32 for f32 {
    fn to_f32_sample(self) -> f32 {
        self.clamp(-1.0, 1.0)
    }
}

impl SampleToF32 for i16 {
    fn to_f32_sample(self) -> f32 {
        self as f32 / i16::MAX as f32
    }
}

impl SampleToF32 for u16 {
    fn to_f32_sample(self) -> f32 {
        (self as f32 - 32768.0) / 32768.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downmix_averages_interleaved_frames() {
        assert_eq!(
            downmix_to_mono(&[1.0, -1.0, 0.5, 0.25], 2),
            vec![0.0, 0.375]
        );
        assert_eq!(downmix_to_mono(&[0.5, -0.5], 1), vec![0.5, -0.5]);
        assert_eq!(downmix_to_mono(&[0.5, -0.5], 0), vec![0.5, -0.5]);
    }

    #[test]
    fn sample_conversion_scales_integer_formats_and_clamps_f32() {
        assert_eq!(32767i16.to_f32_sample(), 1.0);
        assert_eq!((-32767i16).to_f32_sample(), -1.0);
        assert_eq!(65535u16.to_f32_sample(), 32767.0 / 32768.0);
        assert_eq!(0u16.to_f32_sample(), -1.0);
        assert_eq!(2.5f32.to_f32_sample(), 1.0);
        assert_eq!((-2.5f32).to_f32_sample(), -1.0);
    }

    #[test]
    fn capture_error_display_matches_reference_wording() {
        assert_eq!(
            CaptureError::NoDefaultInputDevice.to_string(),
            "no default input microphone"
        );
        assert_eq!(
            CaptureError::BuildInputStream("boom".to_string()).to_string(),
            "build input stream: boom"
        );
        assert_eq!(
            CaptureError::UnsupportedSampleFormat("I24".to_string()).to_string(),
            "unsupported input sample format: I24"
        );
    }
}
