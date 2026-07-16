//! Versioned event envelope shared by firmware and host adapters.

mod generated;

pub use generated::*;

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
}
