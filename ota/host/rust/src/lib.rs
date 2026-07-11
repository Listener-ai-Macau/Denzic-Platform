mod generated;

pub use generated::*;

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TransferReport {
    pub firmware_bytes: usize,
    pub data_writes: u32,
    pub status_reads: u32,
    pub recovered_offsets: u32,
    pub active_link_confirmed: bool,
}

pub trait OtaV1Transport {
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
) -> [u8; CONTROL_BYTES] {
    let mut packet = [0u8; CONTROL_BYTES];
    packet[0..4].copy_from_slice(&MAGIC);
    packet[4] = operation.wire_value();
    packet[5] = PROTOCOL_VERSION;
    packet[8..12].copy_from_slice(&expected_size.to_le_bytes());
    packet[12..14].copy_from_slice(&chunk_payload_bytes.to_le_bytes());
    packet[14..16].copy_from_slice(&window_chunks.to_le_bytes());
    packet
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

pub fn transfer<T: OtaV1Transport>(
    transport: &mut T,
    firmware: &[u8],
    options: TransferOptions,
    mut on_progress: impl FnMut(usize, usize),
) -> Result<TransferReport, String> {
    let options = options.validate()?;
    let expected_size = u32::try_from(firmware.len())
        .map_err(|_| "OTA image exceeds the v1 32-bit size field".to_string())?;
    if expected_size == 0 {
        return Err("OTA image must not be empty".to_string());
    }

    let mut report = TransferReport {
        firmware_bytes: firmware.len(),
        data_writes: 0,
        status_reads: 0,
        recovered_offsets: 0,
        active_link_confirmed: false,
    };
    let begin = control_packet(
        Operation::Begin,
        expected_size,
        options.chunk_payload_bytes,
        options.window_chunks,
    );
    transport
        .write_control(&begin)
        .map_err(|error| format!("OTA begin failed: {error}"))?;

    let initial = read_status_with_retry(transport, options.status_read_attempts, &mut report)
        .map_err(|error| abort_after_error(transport, error))?;
    validate_receiving_status(initial, expected_size)?;
    let chunk_bytes = negotiated_nonzero(options.chunk_payload_bytes, initial.chunk_payload_bytes);
    let configured_window = negotiated_nonzero(options.window_chunks, initial.window_chunks);
    let mut active_link_confirmed = initial.active_link_confirmed();
    let mut window_chunks = limited_window(
        configured_window,
        active_link_confirmed,
        options.inactive_link_window_chunks,
    );
    report.active_link_confirmed = active_link_confirmed;

    let mut confirmed_offset = initial.bytes_written as usize;
    if confirmed_offset > firmware.len() {
        return Err(abort_after_error(
            transport,
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
            transport.write_data(&packet).map_err(|error| {
                abort_after_error(transport, format!("OTA data write failed: {error}"))
            })?;
            report.data_writes += 1;
            send_offset = end;
        }

        let sync = control_packet(Operation::Sync, expected_size, chunk_bytes, window_chunks);
        transport
            .write_control(&sync)
            .map_err(|error| abort_after_error(transport, format!("OTA sync failed: {error}")))?;
        let status = read_status_with_retry(transport, options.status_read_attempts, &mut report)
            .map_err(|error| abort_after_error(transport, error))?;
        validate_receiving_status(status, expected_size)
            .map_err(|error| abort_after_error(transport, error))?;
        let device_offset = status.bytes_written as usize;
        if device_offset > firmware.len() {
            return Err(abort_after_error(
                transport,
                format!(
                    "device reported offset {device_offset} beyond image size {}",
                    firmware.len()
                ),
            ));
        }
        let link_just_became_active = !active_link_confirmed && status.active_link_confirmed();
        active_link_confirmed = status.active_link_confirmed();
        report.active_link_confirmed = active_link_confirmed;
        if device_offset != send_offset {
            report.recovered_offsets += 1;
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
                    options.inactive_link_window_chunks,
                );
            }
        }
        confirmed_offset = device_offset;
        if confirmed_offset <= window_start {
            stalled_windows = stalled_windows.saturating_add(1);
            if stalled_windows >= options.max_stalled_windows {
                return Err(abort_after_error(
                    transport,
                    format!("OTA transfer stalled at offset {confirmed_offset}"),
                ));
            }
        } else {
            stalled_windows = 0;
        }
        on_progress(confirmed_offset, firmware.len());
    }

    let finish = control_packet(Operation::Finish, expected_size, chunk_bytes, window_chunks);
    transport
        .write_finish(&finish)
        .map_err(|error| format!("OTA finish failed: {error}"))?;
    on_progress(firmware.len(), firmware.len());
    Ok(report)
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

fn read_status_with_retry<T: OtaV1Transport>(
    transport: &mut T,
    attempts: u8,
    report: &mut TransferReport,
) -> Result<OtaStatus, String> {
    let mut last_error = None;
    for attempt in 0..attempts {
        report.status_reads += 1;
        match transport
            .read_status()
            .and_then(|bytes| parse_status(&bytes))
        {
            Ok(status) => return Ok(status),
            Err(error) => {
                last_error = Some(error);
                if attempt + 1 < attempts {
                    transport.status_retry_wait(attempt + 1);
                }
            }
        }
    }
    Err(format!(
        "OTA status unavailable after {attempts} attempts: {}",
        last_error.unwrap_or_else(|| "unknown status error".to_string())
    ))
}

fn abort_after_error<T: OtaV1Transport>(transport: &mut T, error: String) -> String {
    let abort = control_packet(Operation::Abort, 0, 0, 0);
    let _ = transport.write_control(&abort);
    error
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockTransport {
        image: Vec<u8>,
        expected_size: u32,
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
    }

    impl MockTransport {
        fn new() -> Self {
            Self {
                image: Vec::new(),
                expected_size: 0,
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
        fn write_control(&mut self, packet: &[u8; CONTROL_BYTES]) -> Result<(), String> {
            assert_eq!(&packet[0..4], &MAGIC);
            match packet[4] {
                OP_BEGIN => {
                    self.expected_size = u32::from_le_bytes(packet[8..12].try_into().unwrap());
                    self.chunk_bytes = u16::from_le_bytes(packet[12..14].try_into().unwrap());
                    self.window_chunks = u16::from_le_bytes(packet[14..16].try_into().unwrap());
                    self.state = STATE_RECEIVING;
                    self.image.clear();
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
            if self.image.len() as u32 != self.expected_size {
                return Err("incomplete image".to_string());
            }
            self.finished = true;
            Ok(())
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
        let packet = control_packet(Operation::Begin, 0x1234_5678, 500, 48);
        assert_eq!(&packet[0..4], b"DOV1");
        assert_eq!(packet[4], OP_BEGIN);
        assert_eq!(packet[5], 1);
        assert_eq!(&packet[8..12], &[0x78, 0x56, 0x34, 0x12]);
        assert_eq!(&packet[12..14], &[0xf4, 0x01]);
        assert_eq!(&packet[14..16], &[0x30, 0x00]);
    }

    #[test]
    fn rejects_invalid_magic() {
        let mut status = MockTransport::new().status_bytes();
        status[0..4].copy_from_slice(b"BAD!");
        assert!(parse_status(&status).is_err());
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
        assert_eq!(report.firmware_bytes, 31);
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
}
