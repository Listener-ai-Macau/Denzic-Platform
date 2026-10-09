//! Companion-compatible IMA-ADPCM (mono 16-bit PCM).
//!
//! Frame layout (same as Companion firmware `companion_ima_adpcm`):
//! - predictor: i16 LE
//! - step_index: u8
//! - reserved: u8 = 0
//! - sample_count: u16 LE
//! - packed nibbles: low nibble = first sample of each pair

const STEP_TABLE: [i16; 89] = [
    7, 8, 9, 10, 11, 12, 13, 14, 16, 17, 19, 21, 23, 25, 28, 31, 34, 37, 41, 45, 50, 55, 60, 66,
    73, 80, 88, 97, 107, 118, 130, 143, 157, 173, 190, 209, 230, 253, 279, 307, 337, 371, 408,
    449, 494, 544, 598, 658, 724, 796, 876, 963, 1060, 1166, 1282, 1411, 1552, 1707, 1878, 2066,
    2272, 2499, 2749, 3024, 3327, 3660, 4026, 4428, 4871, 5358, 5894, 6484, 7132, 7845, 8630,
    9493, 10442, 11487, 12635, 13899, 15289, 16818, 18500, 20350, 22385, 24623, 27086, 29794,
    32767,
];

const INDEX_TABLE: [i8; 16] = [-1, -1, -1, -1, 2, 4, 6, 8, -1, -1, -1, -1, 2, 4, 6, 8];

#[derive(Clone, Debug, Default)]
pub struct ImaAdpcmState {
    pub predictor: i16,
    pub step_index: i8,
}

impl ImaAdpcmState {
    pub fn reset(&mut self) {
        self.predictor = 0;
        self.step_index = 0;
    }
}

fn clamp_i16(value: i32) -> i16 {
    value.clamp(-32768, 32767) as i16
}

fn encode_nibble(state: &mut ImaAdpcmState, sample: i16) -> u8 {
    let step = STEP_TABLE[state.step_index as usize] as i32;
    let mut diff = sample as i32 - state.predictor as i32;
    let mut nibble: u8 = 0;
    if diff < 0 {
        nibble = 8;
        diff = -diff;
    }
    if diff >= step {
        nibble |= 4;
        diff -= step;
    }
    if diff >= (step >> 1) {
        nibble |= 2;
        diff -= step >> 1;
    }
    if diff >= (step >> 2) {
        nibble |= 1;
    }

    let mut delta = step >> 3;
    if (nibble & 4) != 0 {
        delta += step;
    }
    if (nibble & 2) != 0 {
        delta += step >> 1;
    }
    if (nibble & 1) != 0 {
        delta += step >> 2;
    }
    if (nibble & 8) != 0 {
        delta = -delta;
    }
    state.predictor = clamp_i16(state.predictor as i32 + delta);

    let mut step_index = state.step_index as i32 + INDEX_TABLE[(nibble & 0x0f) as usize] as i32;
    step_index = step_index.clamp(0, 88);
    state.step_index = step_index as i8;
    nibble & 0x0f
}

fn decode_nibble(state: &mut ImaAdpcmState, nibble: u8) -> i16 {
    let step = STEP_TABLE[state.step_index as usize] as i32;
    let mut delta = step >> 3;
    if (nibble & 4) != 0 {
        delta += step;
    }
    if (nibble & 2) != 0 {
        delta += step >> 1;
    }
    if (nibble & 1) != 0 {
        delta += step >> 2;
    }
    if (nibble & 8) != 0 {
        delta = -delta;
    }
    state.predictor = clamp_i16(state.predictor as i32 + delta);

    let mut step_index = state.step_index as i32 + INDEX_TABLE[(nibble & 0x0f) as usize] as i32;
    step_index = step_index.clamp(0, 88);
    state.step_index = step_index as i8;
    state.predictor
}

/// Encode one frame. Returns number of bytes written into `out`, or 0 on error.
pub fn encode_frame(
    state: &mut ImaAdpcmState,
    pcm_samples: &[i16],
    out: &mut [u8],
) -> usize {
    if pcm_samples.is_empty() {
        return 0;
    }
    let sample_count = pcm_samples.len();
    if sample_count > u16::MAX as usize {
        return 0;
    }
    let nibble_bytes = (sample_count + 1) / 2;
    let need = 6 + nibble_bytes;
    if out.len() < need {
        return 0;
    }

    let pred = state.predictor as u16;
    out[0] = (pred & 0xff) as u8;
    out[1] = (pred >> 8) as u8;
    out[2] = state.step_index as u8;
    out[3] = 0;
    out[4] = (sample_count & 0xff) as u8;
    out[5] = ((sample_count >> 8) & 0xff) as u8;

    let mut i = 0;
    while i < sample_count {
        let n0 = encode_nibble(state, pcm_samples[i]);
        let n1 = if i + 1 < sample_count {
            encode_nibble(state, pcm_samples[i + 1])
        } else {
            0
        };
        out[6 + i / 2] = (n1 << 4) | n0;
        i += 2;
    }
    need
}

/// Decode one frame into `pcm_out`. Returns sample count written, or 0 on error.
pub fn decode_frame(frame: &[u8], pcm_out: &mut [i16]) -> usize {
    if frame.len() < 6 {
        return 0;
    }
    let mut state = ImaAdpcmState {
        predictor: i16::from_le_bytes([frame[0], frame[1]]),
        step_index: frame[2] as i8,
    };
    if state.step_index < 0 || state.step_index > 88 {
        return 0;
    }
    let samples = u16::from_le_bytes([frame[4], frame[5]]) as usize;
    if samples > pcm_out.len() {
        return 0;
    }
    let nibble_bytes = (samples + 1) / 2;
    if frame.len() < 6 + nibble_bytes {
        return 0;
    }
    let mut written = 0;
    for b in 0..nibble_bytes {
        if written >= samples {
            break;
        }
        let packed = frame[6 + b];
        pcm_out[written] = decode_nibble(&mut state, packed & 0x0f);
        written += 1;
        if written >= samples {
            break;
        }
        pcm_out[written] = decode_nibble(&mut state, (packed >> 4) & 0x0f);
        written += 1;
    }
    written
}

/// Downsample 16 kHz mono i16 → 8 kHz by taking every other sample (half-rate track).
pub fn downsample_half(pcm_16k: &[i16]) -> Vec<i16> {
    pcm_16k.iter().step_by(2).copied().collect()
}

/// Upsample 8 kHz mono i16 → 16 kHz by linear-ish hold (duplicate each sample).
pub fn upsample_x2_hold(pcm_8k: &[i16]) -> Vec<i16> {
    let mut out = Vec::with_capacity(pcm_8k.len() * 2);
    for s in pcm_8k {
        out.push(*s);
        out.push(*s);
    }
    out
}

/// Compress 16 kHz PCM with optional half-rate + IMA frames (512 samples/frame).
/// Returns concatenated frames and original PCM sample count at 16 kHz.
pub fn compress_pcm16k_half_ima(pcm_16k: &[i16], samples_per_frame: usize) -> (Vec<u8>, usize) {
    let half = downsample_half(pcm_16k);
    let mut state = ImaAdpcmState::default();
    let mut blob = Vec::new();
    let frame_samples = samples_per_frame.max(2);
    let mut offset = 0;
    while offset < half.len() {
        let end = (offset + frame_samples).min(half.len());
        let chunk = &half[offset..end];
        let nibble_bytes = (chunk.len() + 1) / 2;
        let mut frame = vec![0u8; 6 + nibble_bytes];
        let n = encode_frame(&mut state, chunk, &mut frame);
        if n == 0 {
            break;
        }
        frame.truncate(n);
        blob.extend_from_slice(&frame);
        offset = end;
    }
    (blob, pcm_16k.len())
}

/// Decode a half-rate IMA blob back to ~16 kHz PCM (hold upsample).
pub fn decompress_half_ima_to_pcm16k(blob: &[u8]) -> Result<Vec<i16>, String> {
    let mut pcm_8k = Vec::new();
    let mut cursor = 0;
    while cursor + 6 <= blob.len() {
        let samples = u16::from_le_bytes([blob[cursor + 4], blob[cursor + 5]]) as usize;
        let nibble_bytes = (samples + 1) / 2;
        let frame_len = 6 + nibble_bytes;
        if cursor + frame_len > blob.len() {
            return Err("truncated IMA frame".into());
        }
        let mut pcm = vec![0i16; samples];
        let n = decode_frame(&blob[cursor..cursor + frame_len], &mut pcm);
        if n != samples {
            return Err(format!("decode wrote {n} expected {samples}"));
        }
        pcm_8k.extend_from_slice(&pcm);
        cursor += frame_len;
    }
    Ok(upsample_x2_hold(&pcm_8k))
}

// --- Capacity constants (shared product planning) ---

/// Raw 16 kHz mono PCM16 store rate.
pub const RAW_PCM16K_BYTES_PER_SEC: u64 = 32_000;
/// Half-rate IMA approximate store rate (4-bit @ 8 kHz) ≈ 8× vs raw.
pub const HALF_IMA_BYTES_PER_SEC: u64 = 4_000;
/// Experimental quarter-rate IMA (4-bit @ 4 kHz) ≈ 16× vs raw. Better capacity,
/// but ASR quality risk — not product default until mechanical acceptance.
pub const QUARTER_IMA_BYTES_PER_SEC: u64 = 2_000;
/// External SPI NOR total on current NUCLEO path.
pub const EXTERNAL_FLASH_TOTAL_BYTES: u64 = 16 * 1024 * 1024;
/// UI external assets (e-paper pack) — product resources, not dual-use for audio.
/// Measured active CEMP (rev20): 396_629 bytes (~19% of this 2 MiB quota).
pub const COMPANION_UI_ASSETS_BYTES: u64 = 2 * 1024 * 1024;
/// Measured mounted CEMP payload (rev20 PackBits pack).
pub const COMPANION_UI_ASSETS_USED_BYTES: u64 = 396_629;
/// Product recording region after flash replan v1 (UI 2 MiB + rec 14 MiB).
pub const COMPANION_RECORDING_REGION_BYTES: u64 = 14 * 1024 * 1024;
/// Alias for replan naming in docs/tools.
pub const COMPANION_RECORDING_REGION_REPLAN_BYTES: u64 = COMPANION_RECORDING_REGION_BYTES;
/// Legacy pre-replan (~11 MiB from 0x500000) — capacity math only.
pub const COMPANION_RECORDING_REGION_LEGACY_BYTES: u64 = 11_534_336;
/// @deprecated aliases → replan 14 MiB.
pub const COMPANION_RECORDING_REGION_DUAL_USE_BYTES: u64 = COMPANION_RECORDING_REGION_BYTES;
pub const COMPANION_RECORDING_REGION_MAX_DUAL_USE_BYTES: u64 = COMPANION_RECORDING_REGION_BYTES;
/// Opus speech planning rates (bytes/sec). Canonical defs also in `opus_codec`.
pub const OPUS_16KBPS_BYTES_PER_SEC: u64 = 2_000;
pub const OPUS_24KBPS_BYTES_PER_SEC: u64 = 3_000;
/// Product default store codec name (host + future board).
pub const PRODUCT_DEFAULT_STORE_CODEC: &str = "opus_16kbps_mono_16khz";

pub fn max_seconds_for_store_rate(region_bytes: u64, bytes_per_sec: u64) -> u64 {
    if bytes_per_sec == 0 {
        return 0;
    }
    region_bytes / bytes_per_sec
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn encode_decode_round_trip_tone() {
        let mut pcm = Vec::new();
        for i in 0..512 {
            let t = i as f32 / 16000.0;
            let s = (2.0 * std::f32::consts::PI * 440.0 * t).sin() * 8000.0;
            pcm.push(s as i16);
        }
        let mut state = ImaAdpcmState::default();
        let mut frame = vec![0u8; 6 + 256];
        let n = encode_frame(&mut state, &pcm, &mut frame);
        assert!(n > 6);
        frame.truncate(n);
        let mut out = vec![0i16; 512];
        let written = decode_frame(&frame, &mut out);
        assert_eq!(written, 512);
        // Lossy codec: require signal still present (energy + bounded MAE).
        let mut sum_err = 0i64;
        let mut energy = 0i64;
        for (a, b) in pcm.iter().zip(out.iter()) {
            let da = *a as i64;
            let db = *b as i64;
            sum_err += (da - db).abs();
            energy += da.abs();
        }
        let mae = sum_err as f64 / pcm.len() as f64;
        assert!(energy > 1_000_000, "tone energy too low");
        assert!(mae < 2500.0, "mae={mae}");
    }

    #[test]
    fn half_ima_compress_ratio_and_capacity() {
        let secs = 10u64;
        let samples = (RAW_PCM16K_BYTES_PER_SEC / 2) * secs; // i16 samples
        let pcm: Vec<i16> = (0..samples)
            .map(|i| ((i % 200) as i16).wrapping_mul(40))
            .collect();
        let (blob, orig) = compress_pcm16k_half_ima(&pcm, 512);
        assert_eq!(orig as u64, samples);
        let raw_bytes = samples * 2;
        let ratio = raw_bytes as f64 / blob.len().max(1) as f64;
        assert!(
            ratio > 6.0,
            "expected ~8:1 half+IMA, got ratio={ratio} raw={raw_bytes} blob={}",
            blob.len()
        );
        let ten_min_blob = HALF_IMA_BYTES_PER_SEC * 600;
        assert!(
            ten_min_blob < COMPANION_RECORDING_REGION_BYTES,
            "10min half-IMA must fit recording region"
        );
        let fit = max_seconds_for_store_rate(COMPANION_RECORDING_REGION_BYTES, HALF_IMA_BYTES_PER_SEC);
        assert!(fit >= 60 * 60, "fit_seconds={fit}");
        let opus16 = max_seconds_for_store_rate(
            COMPANION_RECORDING_REGION_BYTES,
            OPUS_16KBPS_BYTES_PER_SEC,
        );
        assert!(opus16 >= 120 * 60, "opus16_seconds={opus16}");
        assert_eq!(
            COMPANION_UI_ASSETS_BYTES + COMPANION_RECORDING_REGION_BYTES,
            EXTERNAL_FLASH_TOTAL_BYTES
        );
        assert!(
            COMPANION_UI_ASSETS_USED_BYTES < COMPANION_UI_ASSETS_BYTES / 2,
            "UI pack should leave headroom in 2 MiB quota"
        );
        let legacy = max_seconds_for_store_rate(
            COMPANION_RECORDING_REGION_LEGACY_BYTES,
            HALF_IMA_BYTES_PER_SEC,
        );
        assert!(legacy >= 45 * 60 && legacy < fit, "legacy={legacy} replan={fit}");
    }

    #[test]
    fn encode_realtime_budget_on_host() {
        // Proxy for WB M4: host is faster; still require >> realtime for 16k IMA.
        let samples_1s: usize = 16_000;
        let pcm: Vec<i16> = (0..samples_1s * 5)
            .map(|i| ((i % 300) as i16).wrapping_mul(50))
            .collect();
        let start = Instant::now();
        let mut state = ImaAdpcmState::default();
        let mut offset = 0;
        while offset < pcm.len() {
            let end = (offset + 512).min(pcm.len());
            let mut frame = vec![0u8; 6 + 256];
            let n = encode_frame(&mut state, &pcm[offset..end], &mut frame);
            assert!(n > 0);
            offset = end;
        }
        let elapsed = start.elapsed().as_secs_f64().max(1e-9);
        let audio_secs = pcm.len() as f64 / 16_000.0;
        let xrt = audio_secs / elapsed;
        eprintln!("ima_encode_xrt={xrt:.1} audio_s={audio_secs:.2} wall_s={elapsed:.4}");
        assert!(xrt >= 5.0, "encode only {xrt:.1}x realtime");
    }

    #[test]
    fn decompress_half_ima_round_trip_len() {
        let pcm: Vec<i16> = (0..3200).map(|i| (i % 100) as i16 * 30).collect();
        let (blob, _) = compress_pcm16k_half_ima(&pcm, 256);
        let back = decompress_half_ima_to_pcm16k(&blob).expect("decode");
        // Hold upsample after half → length ~ original (±2 for framing)
        assert!((back.len() as i64 - pcm.len() as i64).abs() <= 4);
    }
}
