# BLE pairing capability: adapter boundary

The ble_pairing capability carries the pairing/recovery wire and timing
contract (`protocol/ble_pairing_v1.json`, with normative semantics in
`protocol/ble_pairing_v1.md`), the pure policy decision core, and the
connection-lifecycle orchestration layer. This document states what stays in
the platform and what product adapters must inject.

## What the platform owns

- `embedded/c/src/denzic_ble_pairing_v1.c` — the policy decision core:
  disconnect classification, the no-chase advertising rule, bond-delete
  advertising suppression, recovery pairing window arithmetic, the
  single-shot Swift Pair prompt, identity/IRK policy, and security-failure
  repair (protocol sections 1-7).
- `embedded/c/src/denzic_ble_pairing_v1_orchestration.c` — the orchestration
  layer (protocol section 8): advertising restart routing after disconnect
  and advertising-complete events (shutdown / key-wake / bond-delete
  priority), the disconnect duplicate filter, the advertising payload profile
  plan (Type recovery → Swift Pair → normal), the local IRK reset trigger,
  bond-delete recovery sequencing predicates (warm-up, direct delete vs
  enumeration, unpair API choice, cleanup success, window re-activation), and
  passive reattach evidence classification.
- `host/rust/src/lib.rs` and `host/rust/src/orchestration.rs` — the host
  mirror of both layers.
- `host/rust/src/gatt_cache.rs` — the host-side GATT cache policy table
  (protocol section 9): the cached/uncached attempt sequence per session
  scenario. The table is host-only; the C end carries just the generated
  policy codes.
- Generated constants on both ends from `ble_pairing_v1.json`
  (`denzic_ble_pairing_v1_generated.h`, `generated.rs`).

Both layers are pure: sampled state in, a code out. Neither blocks,
allocates, logs, or touches an OS, timer, or BLE-stack API.

## What product adapters own

- All state storage: connection/advertising flags, recovery window state,
  bond-delete worker bookkeeping, identity-rotation pendings.
- Sampling order: adapters sample each orchestration input at the point
  their own event path requires (e.g. `shutdown_quiesce` before the
  keep-connectable recovery hook, key-wake and bond-delete flags after it).
- Advertising start/stop and payload construction (Swift Pair, Type
  recovery, normal), directed reconnect target selection from the bonded
  peer store, and controller error handling including deferred-return codes.
- Timers: the recovery pairing window timer (arm/rearm/expire), the
  bond-delete disconnect wait, and every debounce/cooldown clock. The
  platform decides *whether* a window has expired; the adapter owns the
  clocks.
- The bond-delete worker itself: task lifecycle, step ordering, and all
  NimBLE/NVS calls. The platform pins which branch each step takes; the
  adapter owns the sequence.
- LED/power/diagnostics output and user-facing wording on both ends.
- Host-side GATT session management: mapping `GattCacheMode` onto the OS
  cache-mode API, running the attempts a policy lists, session open/close,
  retry cadences, pairing prompt UI, PnP/registry cleanup, and poll cadences.
  The platform picks the policy per scenario; the adapter executes it.

## Wiring

- Firmware compiles both C sources from the pinned submodule and pins
  contract constants to the generated header.
- Hosts depend on the `denzic-ble-pairing` crate by path; constants come
  from re-exports, never re-hardcoded.

`tools/verify.ps1` regenerates both ends (`generate_ble_pairing_v1.py
--check`) and runs the C core's ctest suites and the host crate's unit
tests.
