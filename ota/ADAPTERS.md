# OTA Adapter Contract

`denzic_ota_v1` owns the wire protocol and the product-independent decisions
of a firmware update. Product adapters own radios, flash partitions, power
rails, timers, LEDs, and diagnostic sinks.

## Manifest Schema

`protocol/ota_manifest_v2.json` pins the OTA package manifest: the supported
`schema_version` (2; schema_version 1 is retired), required sections and
fields, channel and rollback-method enums, the firmware-version length limit,
the package file name, and the default GATT chunk size. The host Rust crate
(`ota/host/rust/src/manifest.rs`) parses and validates manifests against this
schema; schema constants are generated into `generated_manifest.rs` by
`tools/generate_ota_manifest_v2.py`.

The split between platform and product:

- The platform checks schema facts: field presence and types, channel and
  rollback-method enums, SHA-256 shape, non-zero image size, the protocol
  name, and the `denzic_ota_v1` GATT boundary.
- The product supplies its identity through `OtaManifestPolicy` (package
  type, project name, chunk size, and the label prefixes used in error
  messages) and owns everything after parsing: package-level hash/size
  verification against real bytes, desktop-version gating, hardware-revision
  matching, and downgrade policy.

Firmware packaging/validation tooling must read the schema constants from
`ota_manifest_v2.json` (or the generated surfaces) instead of hardcoding
them, the same way `ota_v1.json` already pins the GATT UUIDs.

## Orchestration Decision Core

`embedded/c/src/denzic_ota_orchestration_v1.c` is a pure-function decision
core. It never touches ESP-IDF, an RTOS, storage, or sensors; the adapter
samples device facts and executes the returned decisions.

- `denzic_ota_orchestration_v1_evaluate_blocker` decides whether an update
  may start, in priority order: in-progress session, pending-verify image,
  product activity (recording, BLE audio, diagnostic export), battery gate
  (default 20%, invalid samples never block), then update-partition
  availability. The adapter injects every input.
- `denzic_ota_orchestration_v1_image_size_accepted` and
  `denzic_ota_orchestration_v1_finish_size_matches` own the known-size
  admission and completion rules; the adapter supplies its unknown-size
  sentinel.
- `denzic_ota_orchestration_v1_inactivity_deadline_us` /
  `denzic_ota_orchestration_v1_inactivity_expired` own the stale-session
  timeout decision (default three minutes); the adapter owns the timer and
  performs the abort.
- `denzic_ota_orchestration_v1_decide_pending_verify` owns the confirm vs
  rollback decision after pending-verify self-checks (POST, BLE readiness,
  keyboard readiness) and returns a rollback reason mask; the adapter maps
  the reason bits onto its diagnostic codes and performs the mark-valid or
  mark-invalid-and-reboot call.

`ota/host/rust/src/orchestration.rs` mirrors the same decisions for host
tooling; keep the two in sync.

## Dual-lane DATA / DATA_B

Products that need Listener-class bulk OTA speed share one dual-lane contract:

- Wire: optional second data characteristic `data_b_uuid` in `ota_v1.json`
  (`GATT_DATA_B_UUID`). Same ATT write semantics as DATA (WWR preferred).
- Device: register DATA_B to the same access path as DATA. Before calling
  `denzic_ota_v1_handle_data`, route packets through
  `denzic_ota_v1_handle_data_ordered` with a product-owned BSS slot array
  (`denzic_ota_v1_reorder_*`). Clear reorder on BEGIN/reset.
- Host: implement `OtaV1Transport::dual_lane_available` / `write_data_b` when
  DATA_B is discovered. Prefer `TransferOptions::dual_lane_bulk(chunk)` (window
  400) over historical tiny windows. The transfer engine alternates lanes.

Listener ESP and Companion STM32WB adapters keep their flash/bootloader paths
product-specific; only the dual-lane wire + reorder + host alternation is
shared.
