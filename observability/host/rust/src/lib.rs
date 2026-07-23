//! Versioned event envelope shared by firmware and host adapters.

use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

mod generated;

pub use generated::*;

static NEXT_HOST_CORRELATION_SEQUENCE: AtomicU64 = AtomicU64::new(1);

/// Creates a non-zero opaque ID for host-originated work.
///
/// The ID combines a wall-clock observation, process identity, and a
/// process-local sequence before mixing. It carries no user or transcript data.
pub fn new_host_correlation_id() -> u64 {
    let clock_nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    let process_id = u64::from(std::process::id());
    let sequence = NEXT_HOST_CORRELATION_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let mut value =
        clock_nanos ^ process_id.rotate_left(23) ^ sequence.wrapping_mul(0x9e37_79b9_7f4a_7c15);
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^= value >> 31;
    value.max(1)
}

/// One decoded BLE diagnostic log chunk notification
/// (`ble_diag_log_gatt` in `observability_v1.json`). `payload` borrows the
/// notified packet and holds `event_count` packed events of
/// `DIAG_LOG_EVENT_WIRE_BYTES` each. The CRC-32/IEEE over the payload is
/// returned for the caller to verify; parsing does not check it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DiagLogChunk<'a> {
    pub event_count: u16,
    pub global_offset: u32,
    pub events_crc32: u32,
    pub payload: &'a [u8],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagLogChunkError {
    TooShort {
        packet_len: usize,
    },
    EmptyChunk,
    PayloadLengthMismatch {
        event_count: u16,
        actual: usize,
        expected: usize,
    },
}

/// Writes the chunk header (event_count u16 LE, global_offset u32 LE,
/// events_crc32 u32 LE) into `out`, which must hold at least
/// `DIAG_LOG_CHUNK_HEADER_BYTES` bytes.
pub fn encode_diag_log_chunk_header(
    out: &mut [u8],
    event_count: u16,
    global_offset: u32,
    events_crc32: u32,
) {
    assert!(out.len() >= DIAG_LOG_CHUNK_HEADER_BYTES);
    out[0..2].copy_from_slice(&event_count.to_le_bytes());
    out[2..6].copy_from_slice(&global_offset.to_le_bytes());
    out[6..10].copy_from_slice(&events_crc32.to_le_bytes());
}

/// Decodes one chunk notification. `global_offset` is the absolute
/// retained-log offset of the first event and stays 32-bit so exports do not
/// truncate after 65535 events.
pub fn parse_diag_log_chunk(packet: &[u8]) -> Result<DiagLogChunk<'_>, DiagLogChunkError> {
    if packet.len() < DIAG_LOG_CHUNK_HEADER_BYTES {
        return Err(DiagLogChunkError::TooShort {
            packet_len: packet.len(),
        });
    }
    let event_count = u16::from_le_bytes([packet[0], packet[1]]);
    if event_count == 0 {
        return Err(DiagLogChunkError::EmptyChunk);
    }
    let payload = &packet[DIAG_LOG_CHUNK_HEADER_BYTES..];
    let expected = usize::from(event_count) * DIAG_LOG_EVENT_WIRE_BYTES;
    if payload.len() != expected {
        return Err(DiagLogChunkError::PayloadLengthMismatch {
            event_count,
            actual: payload.len(),
            expected,
        });
    }
    Ok(DiagLogChunk {
        event_count,
        global_offset: u32::from_le_bytes([packet[2], packet[3], packet[4], packet[5]]),
        events_crc32: u32::from_le_bytes([packet[6], packet[7], packet[8], packet[9]]),
        payload,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_event_is_correlated_and_unambiguous() {
        let event = EventEnvelope::new(
            0x0102_0304_0506_0708,
            7,
            125,
            EventSource::Firmware,
            Capability::Ble,
        );

        assert_eq!(event.contract_version, CONTRACT_VERSION);
        assert_eq!(event.correlation_id, 0x0102_0304_0506_0708);
        assert_eq!(event.event_sequence, 7);
        assert_eq!(event.monotonic_ms, 125);
        assert_eq!(event.ble_lifecycle_state, BleLifecycleState::Unknown);
        assert_eq!(event.error_category, ErrorCategory::None);
        assert_eq!(event.timing_metric, TimingMetric::None);
        assert_eq!(event.timing_value_ms, 0);
    }

    #[test]
    fn recording_latency_and_provider_failure_are_separate_dimensions() {
        let recording = EventEnvelope::new(11, 2, 50, EventSource::Firmware, Capability::Audio)
            .with_ble_lifecycle_state(BleLifecycleState::Recording)
            .with_timing(TimingMetric::EdgeToRecordDispatchMs, 42);
        let provider = EventEnvelope::new(11, 3, 90, EventSource::Provider, Capability::Audio)
            .with_result(CommandResult::Failed)
            .with_error(ErrorCategory::Network);

        assert_eq!(
            recording.timing_metric,
            TimingMetric::EdgeToRecordDispatchMs
        );
        assert_eq!(recording.timing_value_ms, 42);
        assert_eq!(provider.error_category, ErrorCategory::Network);
        assert_eq!(provider.timing_metric, TimingMetric::None);
    }

    #[test]
    fn host_created_correlations_are_nonzero_and_distinct() {
        let first = new_host_correlation_id();
        let second = new_host_correlation_id();

        assert_ne!(first, 0);
        assert_ne!(second, 0);
        assert_ne!(first, second);
    }

    #[test]
    fn diag_log_chunk_round_trips_with_u32_offset() {
        let mut packet = vec![0u8; DIAG_LOG_CHUNK_HEADER_BYTES + 2 * DIAG_LOG_EVENT_WIRE_BYTES];
        encode_diag_log_chunk_header(&mut packet, 2, 70000, 0xdead_beef);
        for byte in &mut packet[DIAG_LOG_CHUNK_HEADER_BYTES..] {
            *byte = 0x5a;
        }

        let chunk = parse_diag_log_chunk(&packet).expect("valid chunk");
        assert_eq!(chunk.event_count, 2);
        assert_eq!(chunk.global_offset, 70000);
        assert_eq!(chunk.events_crc32, 0xdead_beef);
        assert_eq!(chunk.payload.len(), 2 * DIAG_LOG_EVENT_WIRE_BYTES);
        assert!(chunk.payload.iter().all(|byte| *byte == 0x5a));
    }

    #[test]
    fn diag_log_chunk_rejects_malformed_packets() {
        let mut packet = vec![0u8; DIAG_LOG_CHUNK_HEADER_BYTES + DIAG_LOG_EVENT_WIRE_BYTES];

        assert_eq!(
            parse_diag_log_chunk(&packet[..DIAG_LOG_CHUNK_HEADER_BYTES - 1]),
            Err(DiagLogChunkError::TooShort {
                packet_len: DIAG_LOG_CHUNK_HEADER_BYTES - 1
            })
        );
        assert_eq!(
            parse_diag_log_chunk(&packet[..DIAG_LOG_CHUNK_HEADER_BYTES]),
            Err(DiagLogChunkError::EmptyChunk)
        );

        encode_diag_log_chunk_header(&mut packet, 2, 0, 0);
        assert_eq!(
            parse_diag_log_chunk(&packet),
            Err(DiagLogChunkError::PayloadLengthMismatch {
                event_count: 2,
                actual: DIAG_LOG_EVENT_WIRE_BYTES,
                expected: 2 * DIAG_LOG_EVENT_WIRE_BYTES,
            })
        );
    }
}
