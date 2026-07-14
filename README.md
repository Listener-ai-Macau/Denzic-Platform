# Denzic Platform

Shared product cores used by Listener, Companion, and future Denzic devices.
Product repositories pin this repository as a Git submodule. Hardware and OS
details stay in product adapters; wire protocols and state machines live here.
`capabilities.json` is the machine-readable inventory: an active capability is
invalid unless its `protocol`, `host`, and `embedded` layers all exist.

Current modules:

- `ota/host/`: desktop-side package and transfer core.
- `ota/embedded/`: device-side protocol and storage state machine.
- `ota/protocol/`: the single wire contract used by both sides.
- `audio/host/` and `audio/embedded/`: shared VKA1 recording packet and session core.

The portable C recording core under `audio/embedded/c` owns recording state,
session transitions, PCM batch accounting, and format metadata. Product
firmware supplies scheduler, capture, storage, transport, input, and diagnostic
adapters. ESP-IDF I2S, STM32 HAL SAI, FreeRTOS, QSPI, BLE, and GPIO stay out of
the shared core.

Run the repository checks with:

```powershell
pwsh -NoProfile -File .\tools\verify.ps1
```
