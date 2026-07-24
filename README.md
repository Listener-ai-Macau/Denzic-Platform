# Denzic Platform

Shared product cores used by Listener, Companion, and future Denzic devices.
Product repositories pin this repository as a Git submodule. Hardware and OS
details stay in product adapters; wire protocols and state machines live here.
`capabilities.json` is the machine-readable inventory: an active capability is
invalid unless its `protocol`, `host`, and `embedded` layers all exist.

Current modules:

- `ota/host/`: desktop-side package and transfer core, plus the OTA package
  manifest (schema_version 2) parser/validator driven by
  `ota/protocol/ota_manifest_v2.json`.
- `ota/embedded/`: device-side protocol and storage state machine, plus the
  product-independent orchestration decision core
  (`embedded/c/src/denzic_ota_orchestration_v1.c`) for blocker gating, battery
  threshold, image-size admission, inactivity timeout, and pending-verify
  confirm/rollback decisions.
- `ota/protocol/`: the single wire contract used by both sides.
- `audio/host/` and `audio/embedded/`: shared VKA1 recording packet, session, and lossless Rice codec core,
  plus the product-independent BLE audio stream transport engine
  (`embedded/c/src/denzic_audio_transport_v1.c`, mirrored in `host/rust/src/transport_v1.rs`)
  driven by `audio/protocol/audio_transport_v1.json`: media-clock pacing debt,
  true-capacity backpressure hysteresis, the 48-packet replay window, and
  connection-epoch stale-event classification per `audio/protocol/flow_control_v1.md`.
- `observability/`: versioned BLE lifecycle and cross-capability event envelope for firmware and host adapters,
  plus the portable sector-based diagnostic log flash store (`embedded/c/src/denzic_diag_log_store.c`)
  whose storage, time, locking, and output hooks are injected by product adapters,
  and the BLE diagnostic log GATT pull contract with its OS-free chunk codec
  (`embedded/c/src/denzic_diag_log_gatt_v1.c`).
- `device_control/`: transport-neutral discovery, lifecycle, ownership, capability, setting-readback, and command-transaction core. BLE, USB, Wi-Fi, and serial remain product adapters.
- `host_audio/`: host-side microphone capture (cpal), the 16 kHz / mono / 16-bit
  PCM WAV container core, and the ASR provider contracts with an
  OpenAI-compatible batch client, driven by
  `host_audio/protocol/host_audio_v1.json`. Session management, file
  placement, resampling policy, and non-OpenAI ASR providers (Volcengine
  SAUC, local engines) remain product adapters.
- `ble_pairing/`: pairing/recovery policy decision core plus the connection-lifecycle
  orchestration layer (`embedded/c/src/denzic_ble_pairing_v1_orchestration.c`, mirrored in
  `host/rust/src/orchestration.rs`): advertising restart routing, payload profile planning,
  disconnect duplicate filtering, bond-delete recovery sequencing, and reattach evidence
  classification, driven by `ble_pairing/protocol/ble_pairing_v1.json`. Timers, BLE-stack
  calls, LED output, and storage stay in product adapters.
- `tools/release_gate/`: product-independent release-gate toolkit. The
  version-consistency gate (`version_check.py`) runs from a per-product JSON
  config (canonical version source, files that must agree, optional git-tag
  requirement); product repositories call it through thin wrappers that keep
  their original command-line interface and exit codes.

The portable C recording core under `audio/embedded/c` owns recording state,
session transitions, PCM batch accounting, and format metadata. Product
firmware supplies scheduler, capture, storage, transport, input, and diagnostic
adapters. ESP-IDF I2S, STM32 HAL SAI, FreeRTOS, QSPI, BLE, and GPIO stay out of
the shared core.

Run the repository checks with:

```powershell
pwsh -NoProfile -File .\tools\verify.ps1
```
