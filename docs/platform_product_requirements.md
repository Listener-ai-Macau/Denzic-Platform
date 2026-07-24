# Denzic Platform Product Requirements

This registry records owner decisions that govern shared Listener and
Companion product cores. New platform behavior is implemented only after its
owner steering appears here.

| ID | Requirement | Status | Evidence |
| --- | --- | --- | --- |
| PLAT-REQ-001 | Every capability has one canonical JSON contract, generated Rust/C/TypeScript bindings as applicable, a host Rust layer, and an OS-free embedded C layer with explicit tests. Hardware, OS, storage, timer, GATT-stack, GPIO, LED, and product UI effects remain in product adapters. | active | `capabilities.json`; `tools/verify.ps1` |
| PLAT-REQ-002 | Product repositories consume Denzic Platform through a pinned `third_party/denzic-platform` submodule. A shared decision engine must not be reimplemented as a product-private state machine. | active | Companion `COMP-REQ-035`; product submodule pins |
| PLAT-REQ-003 | Complete the current platformization batch in this owner-selected order: (1) adapt Listener Volcengine SAUC streaming ASR to `StreamingAsrProvider` / `StreamingAsrSession` / `AsrEventSink`; (2) migrate Listener host capture to `host_audio::CaptureSession` while preserving liveness watchdog, macOS pause-before-drop, and permission classification; (3) extract boot safety, startup self-test aggregation, and runtime health decisions as `device_health`; (4) extract battery modeling, charge-full debounce, notification policy, and the standard Battery Service `0x180F` contract as `battery`; (5) extract blocker and idle/shutdown decisions as `power_policy`. | active, ordered batch accepted 2026-07-24 | Platform verification plus Listener-Type, Listener-Firmware, and Companion-Type validation commands |
| PLAT-REQ-004 | `device_health`, `battery`, and `power_policy` expose pure deterministic decisions. ESP reset enums, RTC/no-init persistence, FreeRTOS tasks, watchdog feeding, ADC/NVS, BLE characteristic registration/notification, GPIO wake, light sleep/Stop, shutdown, LED, and logging remain adapter effects. | active | C tests use explicit `CHECK` assertions under Release/NDEBUG |
| PLAT-REQ-005 | The protected Companion-Firmware worktree is not edited or repinned while the owner's e-paper commits and uncommitted changes are in flight. Platform APIs and mirrors may be completed now; Companion-Firmware adoption is a separate pin-and-adapter change after the owner clears that worktree boundary. | active | Repository status and later owner approval |
