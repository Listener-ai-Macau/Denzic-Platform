mod generated;
mod generated_manifest;

pub mod manifest;
pub mod orchestration;

pub use generated::*;
pub use generated_manifest::*;

use std::fmt;
use std::time::{Duration, Instant};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Operation {
    Begin,
    Sync,
    Finish,
    Abort,
}

impl Operation {
    fn wire_value(self) -> u8 {
        match self {
            Self::Begin => OP_BEGIN,
            Self::Sync => OP_SYNC,
            Self::Finish => OP_FINISH,
            Self::Abort => OP_ABORT,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct OtaStatus {
    pub state: u8,
    pub last_error: u8,
    pub flags: u8,
    pub bytes_written: u32,
    pub expected_size: u32,
    pub chunk_payload_bytes: u16,
    pub window_chunks: u16,
    pub data_write_count: u32,
}

impl OtaStatus {
    pub fn active_link_confirmed(self) -> bool {
        self.flags & STATUS_FLAG_ACTIVE_LINK_CONFIRMED != 0
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TransferOptions {
    pub chunk_payload_bytes: u16,
    pub window_chunks: u16,
    pub status_read_attempts: u8,
    pub max_stalled_windows: u8,
    pub inactive_link_window_chunks: Option<u16>,
}

impl TransferOptions {
    pub fn validate(self) -> Result<Self, String> {
        if self.chunk_payload_bytes == 0 {
            return Err("OTA chunk payload must be greater than zero".to_string());
        }
        if self.window_chunks == 0 {
            return Err("OTA window must contain at least one chunk".to_string());
        }
        if self.status_read_attempts == 0 {
            return Err("OTA status_read_attempts must be greater than zero".to_string());
        }
        if self.max_stalled_windows == 0 {
            return Err("OTA max_stalled_windows must be greater than zero".to_string());
        }
        if self.inactive_link_window_chunks == Some(0) {
            return Err("OTA inactive-link window must contain at least one chunk".to_string());
        }
        Ok(self)
    }
}

/// Transfer phase an error is attributed to. Transport implementors and host
/// integrations can branch on this instead of parsing message text.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransferPhase {
    Validate,
    Connect,
    Begin,
    DataWrite,
    Sync,
    StatusRead,
    Finish,
    Device,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferError {
    pub phase: TransferPhase,
    pub message: String,
}

impl TransferError {
    fn new(phase: TransferPhase, message: impl Into<String>) -> Self {
        Self {
            phase,
            message: message.into(),
        }
    }
}

impl fmt::Display for TransferError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for TransferError {}

impl From<TransferError> for String {
    fn from(error: TransferError) -> Self {
        error.message
    }
}

/// Wall-clock split of one transfer. `total` covers the whole engine run;
/// the remaining fields accumulate only the time spent inside transport
/// calls, so hosts can separate link time from orchestration overhead.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TransferTimings {
    pub total: Duration,
    pub connect: Duration,
    pub data_write: Duration,
    pub control_write: Duration,
    pub status_read: Duration,
    pub status_retry_wait: Duration,
}

impl TransferTimings {
    pub fn transport_elapsed(&self) -> Duration {
        self.connect
            + self.data_write
            + self.control_write
            + self.status_read
            + self.status_retry_wait
    }

    /// Time spent outside transport calls (encoding, pacing, bookkeeping).
    pub fn non_transfer_elapsed(&self) -> Duration {
        self.total.saturating_sub(self.transport_elapsed())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TransferReport {
    pub firmware_bytes: usize,
    /// Bytes the device had already accepted for this exact image (size +
    /// CRC32 identity) before this run; the engine resumes after them.
    pub resumed_bytes: usize,
    pub data_writes: u32,
    pub status_reads: u32,
    pub recovered_offsets: u32,
    pub active_link_confirmed: bool,
    /// True once the device accepted the finish (confirm) control write.
    pub finish_confirmed: bool,
    pub timings: TransferTimings,
}

impl TransferReport {
    /// Payload bytes per second over the whole run, including windows that
    /// were recovered or resent. Returns `None` when no time was measured.
    pub fn average_payload_bytes_per_second(&self) -> Option<f64> {
        let seconds = self.timings.total.as_secs_f64();
        if seconds > 0.0 {
            Some(self.firmware_bytes as f64 / seconds)
        } else {
            None
        }
    }
}

/// Monotonic time source used by the engine so tests can script time.
pub trait OtaClock {
    fn now(&mut self) -> Duration;
}

pub struct SystemClock {
    origin: Instant,
}

impl Default for SystemClock {
    fn default() -> Self {
        Self {
            origin: Instant::now(),
        }
    }
}

impl OtaClock for SystemClock {
    fn now(&mut self) -> Duration {
        self.origin.elapsed()
    }
}

/// Transport-independent wire operations of one Denzic OTA v1 peer. Hosts
/// implement this over their own BLE stack; all protocol orchestration lives
/// in [`OtaTransferEngine`].
pub trait OtaV1Transport {
    /// Establish the link before the session begins. Transports whose
    /// connection lifecycle is managed by the caller may keep the default.
    fn connect(&mut self) -> Result<(), String> {
        Ok(())
    }

    /// Release the link after the session ends, on success and on failure.
    /// Called exactly once per engine run; must not fail the transfer.
    fn disconnect(&mut self) {}

    fn write_control(&mut self, packet: &[u8; CONTROL_BYTES]) -> Result<(), String>;
    fn write_data(&mut self, packet: &[u8]) -> Result<(), String>;
    fn read_status(&mut self) -> Result<Vec<u8>, String>;

    fn status_retry_wait(&mut self, _attempt: u8) {}

    fn write_finish(&mut self, packet: &[u8; CONTROL_BYTES]) -> Result<(), String> {
        self.write_control(packet)
    }
}

pub fn control_packet(
    operation: Operation,
    expected_size: u32,
    chunk_payload_bytes: u16,
    window_chunks: u16,
    image_crc32: u32,
) -> [u8; CONTROL_BYTES] {
    let mut packet = [0u8; CONTROL_BYTES];
    packet[0..4].copy_from_slice(&MAGIC);
    packet[4] = operation.wire_value();
    packet[5] = PROTOCOL_VERSION;
    packet[8..12].copy_from_slice(&expected_size.to_le_bytes());
    packet[12..14].copy_from_slice(&chunk_payload_bytes.to_le_bytes());
    packet[14..16].copy_from_slice(&window_chunks.to_le_bytes());
    packet[16..20].copy_from_slice(&image_crc32.to_le_bytes());
    packet
}

pub fn crc32_ieee(bytes: &[u8]) -> u32 {
    let mut crc = u32::MAX;
    for byte in bytes {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = 0u32.wrapping_sub(crc & 1);
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }
    !crc
}

pub fn data_packet(offset: u32, payload: &[u8]) -> Vec<u8> {
    let mut packet = Vec::with_capacity(DATA_HEADER_BYTES + payload.len());
    packet.extend_from_slice(&offset.to_le_bytes());
    packet.extend_from_slice(payload);
    packet
}

pub fn parse_status(bytes: &[u8]) -> Result<OtaStatus, String> {
    if bytes.len() < STATUS_BYTES {
        return Err(format!(
            "OTA status is {} bytes; expected at least {STATUS_BYTES}",
            bytes.len()
        ));
    }
    if bytes[0..4] != MAGIC {
        return Err("OTA status magic does not match Denzic OTA v1".to_string());
    }
    if bytes[4] != PROTOCOL_VERSION {
        return Err(format!(
            "OTA status protocol version is {}; expected {PROTOCOL_VERSION}",
            bytes[4]
        ));
    }
    Ok(OtaStatus {
        state: bytes[5],
        last_error: bytes[6],
        flags: bytes[7],
        bytes_written: u32::from_le_bytes(bytes[8..12].try_into().unwrap()),
        expected_size: u32::from_le_bytes(bytes[12..16].try_into().unwrap()),
        chunk_payload_bytes: u16::from_le_bytes(bytes[16..18].try_into().unwrap()),
        window_chunks: u16::from_le_bytes(bytes[18..20].try_into().unwrap()),
        data_write_count: u32::from_le_bytes(bytes[20..24].try_into().unwrap()),
    })
}

/// Transport-independent Denzic OTA v1 transfer orchestration: chunk
/// scheduling, adaptive window pacing, resume by image identity (size +
/// CRC32 negotiated through begin/status), finish confirm, failure
/// attribution, and per-segment timing.
pub struct OtaTransferEngine<'a, T: OtaV1Transport, C: OtaClock> {
    transport: &'a mut T,
    clock: &'a mut C,
    options: TransferOptions,
    report: TransferReport,
}

impl<'a, T: OtaV1Transport, C: OtaClock> OtaTransferEngine<'a, T, C> {
    pub fn new(
        transport: &'a mut T,
        options: TransferOptions,
        clock: &'a mut C,
    ) -> Result<Self, TransferError> {
        let options = options
            .validate()
            .map_err(|message| TransferError::new(TransferPhase::Validate, message))?;
        Ok(Self {
            transport,
            clock,
            options,
            report: TransferReport {
                firmware_bytes: 0,
                resumed_bytes: 0,
                data_writes: 0,
                status_reads: 0,
                recovered_offsets: 0,
                active_link_confirmed: false,
                finish_confirmed: false,
                timings: TransferTimings::default(),
            },
        })
    }

    pub fn run(
        &mut self,
        firmware: &[u8],
        on_progress: impl FnMut(usize, usize),
    ) -> Result<TransferReport, TransferError> {
        let started = self.clock.now();
        let result = self.run_inner(firmware, on_progress);
        self.report.timings.total = self.clock.now().saturating_sub(started);
        self.transport.disconnect();
        result.map(|()| self.report)
    }

    fn run_inner(
        &mut self,
        firmware: &[u8],
        mut on_progress: impl FnMut(usize, usize),
    ) -> Result<(), TransferError> {
        let expected_size = u32::try_from(firmware.len()).map_err(|_| {
            TransferError::new(
                TransferPhase::Validate,
                "OTA image exceeds the v1 32-bit size field",
            )
        })?;
        if expected_size == 0 {
            return Err(TransferError::new(
                TransferPhase::Validate,
                "OTA image must not be empty",
            ));
        }
        let image_crc32 = crc32_ieee(firmware);
        self.report.firmware_bytes = firmware.len();

        self.timed_connect()
            .map_err(|error| self.fail(TransferPhase::Connect, error))?;

        let begin = control_packet(
            Operation::Begin,
            expected_size,
            self.options.chunk_payload_bytes,
            self.options.window_chunks,
            image_crc32,
        );
        self.timed_control_write(&begin)
            .map_err(|error| format!("OTA begin failed: {error}"))
            .map_err(|error| self.fail(TransferPhase::Begin, error))?;

        let initial = self
            .read_status_with_retry()
            .map_err(|error| self.fail_abort(TransferPhase::StatusRead, error))?;
        validate_receiving_status(initial, expected_size)
            .map_err(|error| self.fail(TransferPhase::Device, error))?;
        let chunk_bytes = negotiated_nonzero(
            self.options.chunk_payload_bytes,
            initial.chunk_payload_bytes,
        );
        let configured_window =
            negotiated_nonzero(self.options.window_chunks, initial.window_chunks);
        let mut active_link_confirmed = initial.active_link_confirmed();
        let mut window_chunks = limited_window(
            configured_window,
            active_link_confirmed,
            self.options.inactive_link_window_chunks,
        );
        self.report.active_link_confirmed = active_link_confirmed;

        let mut confirmed_offset = initial.bytes_written as usize;
        self.report.resumed_bytes = confirmed_offset;
        if confirmed_offset > firmware.len() {
            return Err(self.fail_abort(
                TransferPhase::Device,
                format!(
                    "device reported offset {confirmed_offset} beyond image size {}",
                    firmware.len()
                ),
            ));
        }
        let mut stalled_windows = 0u8;
        on_progress(confirmed_offset, firmware.len());

        while confirmed_offset < firmware.len() {
            let window_start = confirmed_offset;
            let mut send_offset = confirmed_offset;
            for _ in 0..window_chunks {
                if send_offset >= firmware.len() {
                    break;
                }
                let end = (send_offset + chunk_bytes as usize).min(firmware.len());
                let packet = data_packet(send_offset as u32, &firmware[send_offset..end]);
                self.timed_data_write(&packet)
                    .map_err(|error| format!("OTA data write failed: {error}"))
                    .map_err(|error| self.fail_abort(TransferPhase::DataWrite, error))?;
                self.report.data_writes += 1;
                send_offset = end;
            }

            let sync = control_packet(
                Operation::Sync,
                expected_size,
                chunk_bytes,
                window_chunks,
                image_crc32,
            );
            self.timed_control_write(&sync)
                .map_err(|error| format!("OTA sync failed: {error}"))
                .map_err(|error| self.fail_abort(TransferPhase::Sync, error))?;
            let status = self
                .read_status_with_retry()
                .map_err(|error| self.fail_abort(TransferPhase::StatusRead, error))?;
            validate_receiving_status(status, expected_size)
                .map_err(|error| self.fail_abort(TransferPhase::Device, error))?;
            let device_offset = status.bytes_written as usize;
            if device_offset > firmware.len() {
                return Err(self.fail_abort(
                    TransferPhase::Device,
                    format!(
                        "device reported offset {device_offset} beyond image size {}",
                        firmware.len()
                    ),
                ));
            }
            let link_just_became_active = !active_link_confirmed && status.active_link_confirmed();
            active_link_confirmed = status.active_link_confirmed();
            self.report.active_link_confirmed = active_link_confirmed;
            if device_offset != send_offset {
                self.report.recovered_offsets += 1;
                if status.last_error == ERROR_OFFSET_MISMATCH {
                    window_chunks = window_chunks.saturating_div(2).max(1);
                }
            } else {
                if active_link_confirmed {
                    window_chunks = if link_just_became_active {
                        configured_window
                    } else {
                        window_chunks.saturating_add(1).min(configured_window)
                    };
                } else {
                    window_chunks = limited_window(
                        configured_window,
                        false,
                        self.options.inactive_link_window_chunks,
                    );
                }
            }
            confirmed_offset = device_offset;
            if confirmed_offset <= window_start {
                stalled_windows = stalled_windows.saturating_add(1);
                if stalled_windows >= self.options.max_stalled_windows {
                    return Err(self.fail_abort(
                        TransferPhase::Device,
                        format!("OTA transfer stalled at offset {confirmed_offset}"),
                    ));
                }
            } else {
                stalled_windows = 0;
            }
            on_progress(confirmed_offset, firmware.len());
        }

        let finish = control_packet(
            Operation::Finish,
            expected_size,
            chunk_bytes,
            window_chunks,
            image_crc32,
        );
        self.timed_finish_write(&finish)
            .map_err(|error| format!("OTA finish failed: {error}"))
            .map_err(|error| self.fail(TransferPhase::Finish, error))?;
        self.report.finish_confirmed = true;
        on_progress(firmware.len(), firmware.len());
        Ok(())
    }

    fn fail(&self, phase: TransferPhase, message: impl Into<String>) -> TransferError {
        TransferError::new(phase, message)
    }

    fn fail_abort(&mut self, phase: TransferPhase, message: impl Into<String>) -> TransferError {
        let abort = control_packet(Operation::Abort, 0, 0, 0, 0);
        let _ = self.transport.write_control(&abort);
        TransferError::new(phase, message)
    }

    fn timed_connect(&mut self) -> Result<(), String> {
        let started = self.clock.now();
        let result = self.transport.connect();
        self.report.timings.connect += self.clock.now().saturating_sub(started);
        result
    }

    fn timed_control_write(&mut self, packet: &[u8; CONTROL_BYTES]) -> Result<(), String> {
        let started = self.clock.now();
        let result = self.transport.write_control(packet);
        self.report.timings.control_write += self.clock.now().saturating_sub(started);
        result
    }

    fn timed_data_write(&mut self, packet: &[u8]) -> Result<(), String> {
        let started = self.clock.now();
        let result = self.transport.write_data(packet);
        self.report.timings.data_write += self.clock.now().saturating_sub(started);
        result
    }

    fn timed_finish_write(&mut self, packet: &[u8; CONTROL_BYTES]) -> Result<(), String> {
        let started = self.clock.now();
        let result = self.transport.write_finish(packet);
        self.report.timings.control_write += self.clock.now().saturating_sub(started);
        result
    }

    fn timed_status_read(&mut self) -> Result<Vec<u8>, String> {
        let started = self.clock.now();
        let result = self.transport.read_status();
        self.report.timings.status_read += self.clock.now().saturating_sub(started);
        result
    }

    fn timed_status_retry_wait(&mut self, attempt: u8) {
        let started = self.clock.now();
        self.transport.status_retry_wait(attempt);
        self.report.timings.status_retry_wait += self.clock.now().saturating_sub(started);
    }

    fn read_status_with_retry(&mut self) -> Result<OtaStatus, String> {
        let attempts = self.options.status_read_attempts;
        let mut last_error = None;
        for attempt in 0..attempts {
            self.report.status_reads += 1;
            match self
                .timed_status_read()
                .and_then(|bytes| parse_status(&bytes))
            {
                Ok(status) => return Ok(status),
                Err(error) => {
                    last_error = Some(error);
                    if attempt + 1 < attempts {
                        self.timed_status_retry_wait(attempt + 1);
                    }
                }
            }
        }
        Err(format!(
            "OTA status unavailable after {attempts} attempts: {}",
            last_error.unwrap_or_else(|| "unknown status error".to_string())
        ))
    }
}

pub fn transfer<T: OtaV1Transport>(
    transport: &mut T,
    firmware: &[u8],
    options: TransferOptions,
    on_progress: impl FnMut(usize, usize),
) -> Result<TransferReport, TransferError> {
    let mut clock = SystemClock::default();
    transfer_with_clock(transport, firmware, options, &mut clock, on_progress)
}

pub fn transfer_with_clock<T: OtaV1Transport, C: OtaClock>(
    transport: &mut T,
    firmware: &[u8],
    options: TransferOptions,
    clock: &mut C,
    on_progress: impl FnMut(usize, usize),
) -> Result<TransferReport, TransferError> {
    OtaTransferEngine::new(transport, options, clock)?.run(firmware, on_progress)
}

fn negotiated_nonzero(requested: u16, reported: u16) -> u16 {
    if reported == 0 {
        requested
    } else {
        requested.min(reported)
    }
}

fn limited_window(configured: u16, active_link: bool, inactive_limit: Option<u16>) -> u16 {
    if active_link {
        configured
    } else {
        inactive_limit.map_or(configured, |limit| configured.min(limit.max(1)))
    }
}

fn validate_receiving_status(status: OtaStatus, expected_size: u32) -> Result<(), String> {
    if status.expected_size != 0 && status.expected_size != expected_size {
        return Err(format!(
            "device reports image size {}, expected {expected_size}",
            status.expected_size
        ));
    }
    if status.state == STATE_ERROR {
        return Err(format!(
            "device entered OTA error state {}",
            status.last_error
        ));
    }
    if status.state != STATE_RECEIVING {
        return Err(format!(
            "device OTA state is {}; expected receiving ({STATE_RECEIVING})",
            status.state
        ));
    }
    if status.last_error != ERROR_NONE && status.last_error != ERROR_OFFSET_MISMATCH {
        return Err(format!("device reports OTA error {}", status.last_error));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    enum LinkEvent {
        Connect,
        Begin,
        Disconnect,
    }

    struct MockTransport {
        image: Vec<u8>,
        expected_size: u32,
        image_crc32: u32,
        chunk_bytes: u16,
        window_chunks: u16,
        state: u8,
        error: u8,
        flags: u8,
        writes: u32,
        transient_status_failures: u8,
        drop_first_data: bool,
        dropped: bool,
        aborted: bool,
        finished: bool,
        fail_finish: bool,
        link_events: Vec<LinkEvent>,
    }

    impl MockTransport {
        fn new() -> Self {
            Self {
                image: Vec::new(),
                expected_size: 0,
                image_crc32: 0,
                chunk_bytes: 0,
                window_chunks: 0,
                state: STATE_IDLE,
                error: ERROR_NONE,
                flags: STATUS_FLAG_ACTIVE_LINK_CONFIRMED,
                writes: 0,
                transient_status_failures: 0,
                drop_first_data: false,
                dropped: false,
                aborted: false,
                finished: false,
                fail_finish: false,
                link_events: Vec::new(),
            }
        }

        fn status_bytes(&self) -> Vec<u8> {
            let mut bytes = vec![0u8; STATUS_BYTES];
            bytes[0..4].copy_from_slice(&MAGIC);
            bytes[4] = PROTOCOL_VERSION;
            bytes[5] = self.state;
            bytes[6] = self.error;
            bytes[7] = self.flags;
            bytes[8..12].copy_from_slice(&(self.image.len() as u32).to_le_bytes());
            bytes[12..16].copy_from_slice(&self.expected_size.to_le_bytes());
            bytes[16..18].copy_from_slice(&self.chunk_bytes.to_le_bytes());
            bytes[18..20].copy_from_slice(&self.window_chunks.to_le_bytes());
            bytes[20..24].copy_from_slice(&self.writes.to_le_bytes());
            bytes
        }
    }

    impl OtaV1Transport for MockTransport {
        fn connect(&mut self) -> Result<(), String> {
            self.link_events.push(LinkEvent::Connect);
            Ok(())
        }

        fn disconnect(&mut self) {
            self.link_events.push(LinkEvent::Disconnect);
        }

        fn write_control(&mut self, packet: &[u8; CONTROL_BYTES]) -> Result<(), String> {
            assert_eq!(&packet[0..4], &MAGIC);
            match packet[4] {
                OP_BEGIN => {
                    self.link_events.push(LinkEvent::Begin);
                    let expected_size = u32::from_le_bytes(packet[8..12].try_into().unwrap());
                    let image_crc32 = u32::from_le_bytes(packet[16..20].try_into().unwrap());
                    let same_image = self.state == STATE_RECEIVING
                        && self.expected_size == expected_size
                        && self.image_crc32 == image_crc32;
                    self.expected_size = expected_size;
                    self.image_crc32 = image_crc32;
                    self.chunk_bytes = u16::from_le_bytes(packet[12..14].try_into().unwrap());
                    self.window_chunks = u16::from_le_bytes(packet[14..16].try_into().unwrap());
                    self.state = STATE_RECEIVING;
                    if !same_image {
                        self.image.clear();
                    }
                }
                OP_SYNC => {}
                OP_ABORT => self.aborted = true,
                value => return Err(format!("unexpected control operation {value}")),
            }
            Ok(())
        }

        fn write_data(&mut self, packet: &[u8]) -> Result<(), String> {
            let offset = u32::from_le_bytes(packet[0..4].try_into().unwrap()) as usize;
            if self.drop_first_data && !self.dropped {
                self.dropped = true;
                return Ok(());
            }
            if offset != self.image.len() {
                self.error = ERROR_OFFSET_MISMATCH;
                return Ok(());
            }
            self.image.extend_from_slice(&packet[4..]);
            self.error = ERROR_NONE;
            self.writes += 1;
            Ok(())
        }

        fn read_status(&mut self) -> Result<Vec<u8>, String> {
            if self.transient_status_failures > 0 {
                self.transient_status_failures -= 1;
                return Err("transient GATT read".to_string());
            }
            Ok(self.status_bytes())
        }

        fn write_finish(&mut self, packet: &[u8; CONTROL_BYTES]) -> Result<(), String> {
            assert_eq!(packet[4], OP_FINISH);
            if self.fail_finish {
                return Err("link dropped during reboot".to_string());
            }
            if self.image.len() as u32 != self.expected_size {
                return Err("incomplete image".to_string());
            }
            self.finished = true;
            Ok(())
        }
    }

    struct FakeClock {
        now: Duration,
        step: Duration,
    }

    impl FakeClock {
        fn new(step: Duration) -> Self {
            Self {
                now: Duration::ZERO,
                step,
            }
        }
    }

    impl OtaClock for FakeClock {
        fn now(&mut self) -> Duration {
            let now = self.now;
            self.now += self.step;
            now
        }
    }

    fn options() -> TransferOptions {
        TransferOptions {
            chunk_payload_bytes: 5,
            window_chunks: 3,
            status_read_attempts: 3,
            max_stalled_windows: 3,
            inactive_link_window_chunks: Some(1),
        }
    }

    #[test]
    fn encodes_the_single_v1_wire_contract() {
        let packet = control_packet(Operation::Begin, 0x1234_5678, 500, 48, 0x89ab_cdef);
        assert_eq!(&packet[0..4], b"DOV1");
        assert_eq!(packet[4], OP_BEGIN);
        assert_eq!(packet[5], 1);
        assert_eq!(&packet[8..12], &[0x78, 0x56, 0x34, 0x12]);
        assert_eq!(&packet[12..14], &[0xf4, 0x01]);
        assert_eq!(&packet[14..16], &[0x30, 0x00]);
        assert_eq!(&packet[16..20], &[0xef, 0xcd, 0xab, 0x89]);
        assert_eq!(crc32_ieee(b"123456789"), 0xcbf4_3926);
    }

    #[test]
    fn rejects_invalid_magic() {
        let mut status = MockTransport::new().status_bytes();
        status[0..4].copy_from_slice(b"BAD!");
        assert!(parse_status(&status).is_err());
    }

    #[test]
    fn rejects_invalid_options_before_touching_the_transport() {
        let firmware: Vec<u8> = (0..7).collect();
        let mut transport = MockTransport::new();
        let mut invalid = options();
        invalid.window_chunks = 0;
        let error = transfer(&mut transport, &firmware, invalid, |_, _| {}).unwrap_err();
        assert_eq!(error.phase, TransferPhase::Validate);
        assert!(transport.link_events.is_empty());
    }

    #[test]
    fn transfers_with_status_retry_and_progress() {
        let firmware: Vec<u8> = (0..31).collect();
        let mut transport = MockTransport::new();
        transport.transient_status_failures = 1;
        let mut progress = Vec::new();
        let report = transfer(&mut transport, &firmware, options(), |done, total| {
            progress.push((done, total));
        })
        .unwrap();
        assert_eq!(transport.image, firmware);
        assert!(transport.finished);
        assert!(report.finish_confirmed);
        assert_eq!(report.firmware_bytes, 31);
        assert_eq!(report.resumed_bytes, 0);
        assert!(report.active_link_confirmed);
        assert!(report.status_reads >= 2);
        assert_eq!(progress.last(), Some(&(31, 31)));
    }

    #[test]
    fn resumes_from_device_reported_offset() {
        let firmware: Vec<u8> = (0..23).collect();
        let mut transport = MockTransport::new();
        transport.drop_first_data = true;
        let report = transfer(&mut transport, &firmware, options(), |_, _| {}).unwrap();
        assert_eq!(transport.image, firmware);
        assert!(transport.finished);
        assert!(report.recovered_offsets >= 1);
    }

    #[test]
    fn resumes_an_existing_same_image_session_without_erasing_device_progress() {
        let firmware: Vec<u8> = (0..31).collect();
        let existing_bytes = 15usize;
        let mut transport = MockTransport::new();
        transport
            .image
            .extend_from_slice(&firmware[..existing_bytes]);
        transport.expected_size = firmware.len() as u32;
        transport.image_crc32 = crc32_ieee(&firmware);
        transport.chunk_bytes = options().chunk_payload_bytes;
        transport.window_chunks = options().window_chunks;
        transport.state = STATE_RECEIVING;

        let mut progress = Vec::new();
        let report = transfer(&mut transport, &firmware, options(), |done, total| {
            progress.push((done, total));
        })
        .unwrap();

        assert_eq!(progress.first(), Some(&(existing_bytes, firmware.len())));
        assert_eq!(transport.image, firmware);
        assert!(transport.finished);
        assert_eq!(report.resumed_bytes, existing_bytes);
        assert!(report.data_writes < 7);
    }

    #[test]
    fn byte_counts_and_rate_follow_transfer_semantics() {
        // 31 bytes, 5-byte chunks, 15 already on the device: exactly 4 data
        // writes must cover the remaining 16 bytes.
        let firmware: Vec<u8> = (0..31).collect();
        let existing_bytes = 15usize;
        let mut transport = MockTransport::new();
        transport
            .image
            .extend_from_slice(&firmware[..existing_bytes]);
        transport.expected_size = firmware.len() as u32;
        transport.image_crc32 = crc32_ieee(&firmware);
        transport.chunk_bytes = options().chunk_payload_bytes;
        transport.window_chunks = options().window_chunks;
        transport.state = STATE_RECEIVING;

        let mut clock = FakeClock::new(Duration::from_millis(100));
        let report =
            transfer_with_clock(&mut transport, &firmware, options(), &mut clock, |_, _| {})
                .unwrap();

        assert_eq!(report.firmware_bytes, 31);
        assert_eq!(report.resumed_bytes, existing_bytes);
        assert_eq!(report.data_writes, 4);
        assert_eq!(report.status_reads, 3);
        let timings = report.timings;
        assert!(timings.data_write > Duration::ZERO);
        assert!(timings.control_write > Duration::ZERO);
        assert!(timings.status_read > Duration::ZERO);
        assert_eq!(timings.status_retry_wait, Duration::ZERO);
        assert_eq!(
            timings.non_transfer_elapsed(),
            timings.total - timings.transport_elapsed(),
            "non-transfer time must be the remainder outside transport calls"
        );
        assert!(timings.total >= timings.transport_elapsed());
        let rate = report
            .average_payload_bytes_per_second()
            .expect("a timed transfer must report a payload rate");
        let expected_rate = 31.0 / timings.total.as_secs_f64();
        assert!(
            (rate - expected_rate).abs() < f64::EPSILON,
            "rate must derive from payload bytes over total elapsed"
        );
    }

    #[test]
    fn status_retry_wait_is_timed_separately_from_reads() {
        let firmware: Vec<u8> = (0..11).collect();
        let mut transport = MockTransport::new();
        transport.transient_status_failures = 1;
        let mut clock = FakeClock::new(Duration::from_millis(50));
        let report =
            transfer_with_clock(&mut transport, &firmware, options(), &mut clock, |_, _| {})
                .unwrap();
        assert_eq!(transport.image, firmware);
        assert_eq!(report.timings.status_retry_wait, Duration::from_millis(50));
        assert!(report.timings.status_read >= Duration::from_millis(100));
    }

    #[test]
    fn connect_precedes_begin_and_disconnect_closes_every_run() {
        let firmware: Vec<u8> = (0..9).collect();
        let mut transport = MockTransport::new();
        transfer(&mut transport, &firmware, options(), |_, _| {}).unwrap();
        assert_eq!(
            transport.link_events,
            vec![LinkEvent::Connect, LinkEvent::Begin, LinkEvent::Disconnect],
            "engine must connect before begin and disconnect after a successful run"
        );

        let mut failing = MockTransport::new();
        failing.fail_finish = true;
        let error = transfer(&mut failing, &firmware, options(), |_, _| {}).unwrap_err();
        assert_eq!(error.phase, TransferPhase::Finish);
        assert!(
            failing.link_events.contains(&LinkEvent::Disconnect),
            "engine must disconnect even when the transfer fails"
        );
    }

    #[test]
    fn finish_failure_is_attributed_and_never_aborted() {
        let firmware: Vec<u8> = (0..17).collect();
        let mut transport = MockTransport::new();
        transport.fail_finish = true;
        let error = transfer(&mut transport, &firmware, options(), |_, _| {}).unwrap_err();
        assert_eq!(error.phase, TransferPhase::Finish);
        assert!(
            error.message.contains("OTA finish failed"),
            "finish errors must keep their phase prefix: {}",
            error.message
        );
        assert!(
            !transport.aborted,
            "a failed finish confirm must not abort: the device may already be rebooting"
        );
        assert!(!transport.finished);
    }

    #[test]
    fn data_write_failure_aborts_and_attributes_the_phase() {
        struct FailingDataTransport {
            inner: MockTransport,
        }

        impl OtaV1Transport for FailingDataTransport {
            fn write_control(&mut self, packet: &[u8; CONTROL_BYTES]) -> Result<(), String> {
                self.inner.write_control(packet)
            }

            fn write_data(&mut self, _packet: &[u8]) -> Result<(), String> {
                Err("GATT write rejected".to_string())
            }

            fn read_status(&mut self) -> Result<Vec<u8>, String> {
                self.inner.read_status()
            }
        }

        let firmware: Vec<u8> = (0..13).collect();
        let mut transport = FailingDataTransport {
            inner: MockTransport::new(),
        };
        let error = transfer(&mut transport, &firmware, options(), |_, _| {}).unwrap_err();
        assert_eq!(error.phase, TransferPhase::DataWrite);
        assert!(error.message.contains("OTA data write failed"));
        assert!(
            transport.inner.aborted,
            "mid-transfer failures must abort the device session"
        );
        let as_string: String = error.into();
        assert!(as_string.contains("OTA data write failed"));
    }
}
