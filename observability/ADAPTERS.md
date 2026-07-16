# Observability v1 Adapter Contract

Firmware and Type emit one JSON object per event using the field names in
`denzic_observability_v1_event_t` and `EventEnvelope`. The envelope is
append-only: adapters do not replace a causal event with a rendered message.

## Correlation

- Firmware creates `correlation_id` once for a physical input, BLE recovery, or
  OTA operation and includes it in every event it emits for that operation.
- Type preserves the received `correlation_id`; Type-created work starts a new
  non-zero identifier and carries it through transport and provider events.
- `event_sequence` is monotonically increasing only within one source and one
  `correlation_id`. It orders local events but is not a global clock.
- `monotonic_ms` is source-local. Compare firmware and Type events by shared
  `correlation_id` and lifecycle order, never by subtracting their clocks.

## Timing And Attribution

Each measured latency is emitted as `timing_metric` plus `timing_value_ms`.
Use exactly one of: `edge_to_record_dispatch_ms`, `ble_recovery_ms`,
`audio_first_packet_ms`, `preview_latency_ms`, `final_transcription_ms`, or
`ota_transfer_ms`. A missing measurement uses `none` with value zero.

`source`, `capability`, `ble_lifecycle_state`, `command_result`, and
`error_category` identify the root-cause domain. Classify device capture or
firmware errors as `device`; GATT/HID/audio-link errors as `transport`; desktop
state errors as `host`; model failures as `provider`; connectivity failures as
`network`; schema mismatches as `protocol`; and exhaustion/time-budget failures
as `resource`.

## Adapter Targets

- Firmware includes `observability/embedded/c/include/denzic_observability_v1_generated.h`.
- Type consumes `observability/host/rust/src/lib.rs` and serializes the Rust
  `EventEnvelope` without renaming its fields.
- Both adapters validate their declared audio, OTA, BLE, and observability
  versions with `tools/verify_adapter_compatibility.py` before integration.
