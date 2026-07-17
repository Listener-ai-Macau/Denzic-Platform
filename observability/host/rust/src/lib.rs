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
}
