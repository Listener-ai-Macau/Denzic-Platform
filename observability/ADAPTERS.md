# Observability v1 Adapter Contract

Firmware and Type emit one JSON object per event using the field names in
`denzic_observability_v1_event_t` and `EventEnvelope`. The envelope is
append-only: adapters do not replace a causal event with a rendered message.

## Correlation

- Firmware creates `correlation_id` once for a physical input, BLE recovery, or
  OTA operation and includes it in every event it emits for that operation.
- Type preserves the received `correlation_id`; Type-created work starts a new
  non-zero host-generated opaque identifier for every operation and carries it
  through transport and provider events. A process restart must not reuse the
  first operation's identifier.
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

## BLE Diagnostic Log GATT Service

The `ble_diag_log_gatt` section of `observability_v1.json` owns the GATT
surface used to pull the retained diagnostic log over BLE. Generated
constants (`DENZIC_OBSERVABILITY_V1_DIAG_LOG_*` in C, `DIAG_LOG_*` in Rust)
are the single source for the four UUIDs and the two wire sizes; adapters
must not restate them.

- Service/control/data/count characteristics use the contract UUIDs. Control
  is write-only JSON, data is notify-only, and count is read-only.
- Count reads return `{"count":N}` with the number of retained events.
- Each event is a packed 24-byte frame: timestamp u32, source u16, event u8,
  severity u8, arg1-4 u32 (the same layout the flash store retains).
- Pull semantics: the host subscribes to data notifications, writes
  `{"op":"start"}` to control, then for each chunk writes
  `{"op":"read","offset":N}` and receives one data notification; it writes
  `{"op":"stop"}` to end the session.
- Each data notification carries a 10-byte chunk header — event_count u16 LE,
  global_offset u32 LE, events_crc32 u32 LE (CRC-32/IEEE over the packed
  event payload) — followed by event_count packed 24-byte events.
  `global_offset` is the absolute retained-log offset of the first event in
  the chunk and stays 32-bit so exports do not truncate after 65535 events.
- Chunk encoding/decoding lives in
  `observability/embedded/c/src/denzic_diag_log_gatt_v1.c`
  (`denzic_diag_log_gatt_v1_encode_chunk_header` /
  `denzic_diag_log_gatt_v1_decode_chunk`) and in the host crate
  (`encode_diag_log_chunk_header` / `parse_diag_log_chunk`). Adapters supply
  their platform CRC primitive and the MTU/concurrency policy.
