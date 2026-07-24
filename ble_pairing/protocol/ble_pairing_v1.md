# BLE Pairing Decision Contract v1

Machine-checkable semantics for the pairing/recovery policy core
(`ble_pairing_v1.json`). This document is normative for both the embedded
decision core (`denzic_ble_pairing_v1.c`) and the host mirror crate
(`denzic-ble-pairing`). All inputs are plain numbers and booleans; the core
has no OS, timer, or BLE-stack dependency. Every side effect (advertising
start/stop, identity rotation, NVS access, timers, logging) stays in the
calling product adapter.

Protocol constants referenced below live in `ble_pairing_v1.json`:

| Name | Value | Meaning |
| --- | --- | --- |
| `recovery_pairing_window_ms` | 120,000 | Default recovery pairing window lifetime. |
| `recovery_swift_pair_prompt_window_ms` | 45,000 | Bounded Swift Pair prompt window inside the recovery window. |
| `first_pairing_window_ms` | 0 | First-pairing Swift Pair window (0 = disabled by default). |
| `swift_pair_adv_min_restart_ms` | 1,000 | Minimum advertising restart duration while a prompt is active. |
| `recovery_bond_delete_wait_ms` | 2,000 | Max wait for disconnect before async local bond cleanup gives up. |
| `recovery_advertising_command_target_ms` | 250 | Recovery advertising command acceptance gate (e.g. EC11 path). |
| `hci.nimble_hci_status_base` | 512 | Base added to an HCI error to form the host-stack status code (0x200). |
| `hci.remote_user_terminated_reason` | 19 | HCI "Remote User Terminated Connection" (0x13). |
| `hci.host_deliberate_disconnect_status` | 531 | 512 + 19: status observed when the host deleted the pairing. |
| `disconnect_duplicate_filter_ms` | 750 | Window in which a repeated disconnect event for the same connection is a stack echo. |

## 1. Disconnect classification

| locally requested? | status code | Classification |
| --- | --- | --- |
| yes | any | `local_request` |
| no | `host_deliberate_disconnect_status` (531) | `host_deliberate_unpair` |
| no | anything else | `transient_link_loss` |

The host stack cannot distinguish a locally requested terminate that carries
the 0x13 reason from a remote one; the caller passes `locally_requested`
from its own context (a recovery terminate is intercepted by the recovery
flow before classification runs).

## 2. Advertising after disconnect (no-chase rule)

| bond delete active? | classification | Action |
| --- | --- | --- |
| yes | any | `suppress` (worker owns the single recovery advertising start) |
| no | `host_deliberate_unpair` | `undirected_only` — never chase a host that deliberately removed the pairing with directed advertising |
| no | `transient_link_loss` | `directed_reconnect` — ordinary directed reconnect intent |

A directed intent is additionally gated at advertising start:
`directed_reconnect_allowed = directed_pending AND NOT low_power AND NOT
pairing_window_open` (and a bonded peer address must exist). During an open
pairing window advertising is always undirected and connectable.

## 3. Bond-delete advertising suppression

`advertising_allowed_during_bond_delete(active, warmup_permitted) =
active ? warmup_permitted : false`. While an async local bond delete is
pending or in progress, advertising requests are deferred unless the caller
explicitly permits a bounded non-connectable warm-up. The worker performs
cleanup, then identity rotation (below), then the single advertising start.

## 4. Recovery pairing window lifecycle

- `window_remaining_ms(opened_at, window_ms, now)`: 0 when `opened_at <= 0`
  or `window_ms <= 0`; `window_ms` when the clock ran backwards
  (`now < opened_at`); 0 once elapsed >= `window_ms`; otherwise
  `window_ms - elapsed`.
- Expiry (remaining hits 0) closes the window.
- `window_close_on_secure(type_controlled, window_open)`: a Type-controlled
  recovery with the window still open yields `keep_open` — the window waits
  for the Type audio-ready signal; everything else yields `close_now`.
- `window_close_on_type_audio_ready(window_open, waiting_for_disconnect,
  desc_valid, encrypted, bonded)` yields `close_now` only when the window is
  open, the recovery is not still waiting for the pre-reset connection to
  drop, the connection descriptor is valid, and the link is encrypted or
  bonded. Otherwise `keep_open`: an audio-ready signal on an insecure or
  stale connection must never close the window.

## 5. Swift Pair prompt evaluation

Inside an open recovery window the prompt is single-shot:

| suppressed? | consumed? | prompt window <= 0? | remaining <= 0? | Evaluation |
| --- | --- | --- | --- | --- |
| yes | — | — | — | `suppressed` (silent Type-controlled recovery; consumes the shot) |
| no | yes | — | — | `consumed` |
| no | no | yes | — | `disabled` (consumes the shot) |
| no | no | no | yes | `expired` (consumes the shot) |
| no | no | no | no | `active` with the remaining milliseconds |

`swift_pair_adv_duration_ms(remaining)` clamps a bounded prompt
advertisement: at least `swift_pair_adv_min_restart_ms`, at most INT32_MAX.

## 6. Identity policy

- `type_controlled_recovery(request, type_link_ready, type_host_recent,
  connected) = request OR type_link_ready OR (type_host_recent AND
  connected)`. A recent-host marker alone is not ownership evidence once
  that host's pairing was deleted; only a live link or an explicit request
  keeps the stable Type identity.
- `identity_for_recovery(type_controlled, connected)`:

  | type_controlled? | connected? | Action |
  | --- | --- | --- |
  | yes | — | `keep_stable` (no IRK rotation; PairAsync reuses the identity) |
  | no | yes | `defer_rotate_until_disconnect` (rotation is unsafe mid-link) |
  | no | no | `rotate_fresh` (reset local IRK first when no bonds remain, then apply a fresh random identity) |

- `rotate_before_advertising(identity_rotate_pending)`: when rotation was
  deferred, it must run after the disconnect (and after local bond cleanup)
  and before the recovery advertising start — never rotate while a bond
  delete is mid-flight and never advertise the stale identity afterwards.
- `refresh_existing_pairing_window(window_open, bonded_count, connected,
  force_fresh_identity)`: an already-open window with no bonds, no link, and
  no forced fresh identity is refreshed in place with the stable identity
  instead of tearing down and reopening.

## 7. Security-failure repair

`security_failure_action(pairing_window_open)`: an encryption failure inside
an open pairing window yields `retry_within_window` — keep pairing
advertising available for the host's retry and terminate the insecure link
without restarting repair. Outside a window it yields `open_repair_window` —
open a full pairing reset and terminate the insecure connection.

## 8. Connection-lifecycle orchestration

The orchestration layer (`denzic_ble_pairing_v1_orchestration.c`, mirrored by
`orchestration.rs` in the host crate) sequences what the policy tables decide.
Every function remains pure: the caller samples each input at the point its
own event path requires and executes every side effect itself.

### 8.1 Advertising restart routing

After a disconnect, restart routing is a fixed priority:

| shutdown quiesce? | key-wake-only? | bond delete active? | Decision |
| --- | --- | --- | --- |
| yes | — | — | `suppress_shutdown` |
| no | yes | — | `suppress_key_wake` |
| no | no | yes | `defer_bond_delete` (the worker owns the single recovery advertising start) |
| no | no | no | `restart` |

The caller samples `shutdown_quiesce` before running its keep-connectable
recovery hook and the other inputs after it, so the hook's flag clearing is
honoured exactly once. After an advertising-complete event the same suppress
priority applies, but a bond delete never defers: the completion event
belongs to the worker-owned advertising cycle.

### 8.2 Disconnect duplicate filter

A repeated disconnect event for the same connection handle and reason inside
`disconnect_duplicate_filter_ms` is a stack echo:
`elapsed >= 0 AND elapsed < filter_ms`. A clock moving backwards is never a
duplicate. Handle/reason comparison stays in the caller.

### 8.3 Advertising payload profile plan

1. A Type-controlled recovery payload is built first, but only when no Swift
   Pair prompt competes: `try_type_recovery = type_recovery_requested AND NOT
   swift_pair_requested`.
2. Swift Pair is tried next: `try_swift_pair = NOT type_recovery_enabled AND
   swift_pair_requested`.
3. Final selection: `swift_pair` when its payload built, else `type_recovery`
   when its payload built, else `normal`. Payload construction stays in the
   adapter and may fail.

### 8.4 Local IRK reset trigger

`irk_reset_after_disconnect = pairing_window_open AND identity_rotate_pending
AND bond_lookup_ok AND bonded_peer_count == 0`. A failed bond lookup must
never reset the IRK on an unknown bond state.

### 8.5 Bond-delete recovery sequencing

- Warm-up advertising before cleanup and direct single-peer delete both apply
  only for a Type-controlled recovery with a known current peer; anything
  else falls back to enumerating bonded peers.
- During enumeration a native recovery removes peers through the GAP unpair
  API (so the local IRK rotates with the final bond); a Type-controlled
  recovery deletes peer records and keeps the local identity for re-pairing.
- Cleanup succeeded when a direct delete ran, or the enumeration lookup and
  every per-peer delete returned 0.
- After the recovery advertising start the pairing window guard is
  re-activated only when it lapsed while the window is still open:
  `NOT power_blocker_active AND pairing_window_active`.

### 8.6 Passive reattach evidence

`reattach_evidence_ready = (paired_devices_visible AND native_hid_present) OR
fresh_native_hid_after_baseline`. An existing host pairing only counts as
evidence with a live native HID endpoint; a fresh HID address after the
monitoring baseline is evidence on its own. Address-set diffing stays in the
adapter.

## 9. Host GATT cache policy

The host-side cache policy table (`gatt_cache.rs` in the host crate) decides
when a host (central) may serve GATT service/characteristic access from the
system cache and when it must force fresh discovery from the peer. A
peripheral stack has no GATT client cache concept, so this layer exists only
in the host mirror; the C end carries just the generated policy codes.

### 9.1 Cache policies

| Policy | Code | Attempt sequence |
| --- | --- | --- |
| `cached_only` | 1 | cached |
| `uncached_only` | 2 | uncached |
| `uncached_first` | 3 | uncached, then cached as fallback |
| `cached_first` | 4 | cached, then uncached as fallback |

The adapter maps each mode onto its OS cache-mode API and runs the attempts
in the listed order; it never reorders or extends a sequence.

### 9.2 Scenario table

| Scenario constant | Policy |
| --- | --- |
| `RECENT_PAIRING_NOTIFY_CACHE_POLICY` | `cached_first` — a just-completed pairing may leave a valid system cache |
| `KNOWN_ADDRESS_NOTIFY_CACHE_POLICY` | `uncached_first` — default notify discovery posture over known addresses |
| `DEVICE_NOTIFY_CACHE_POLICY` | `uncached_first` — notify open by one exact address |
| `PERSISTED_BOND_NOTIFY_CACHE_POLICY` | `cached_first` — a persisted OS-level bond can hold a valid cache while fresh queries temporarily fail (link rehydration) |
| `POST_CONFIRM_NOTIFY_CACHE_POLICY` | `cached_first` — a confirmed image retains its GATT schema; uncached fallback covers Service Changed |
| `SERVICE_REACHABILITY_PROBE_CACHE_POLICY` | `cached_first` — fast probe of an already-known service |
| `STATUS_PROBE_CACHE_POLICY` | `cached_first` — bounded status-target open |
| `CONTROL_WRITE_CACHE_POLICY` | `uncached_first` — control writes need fresh handles, cache only as fallback |
| `DIAGNOSTIC_CACHE_POLICY` | `uncached_only` — diagnostics always read fresh |
| `DEADLINE_SERVICE_ENDPOINT_CACHE_POLICY` | `cached_first` — the deadline-bounded endpoint open rehydrates the cache first to fit its budget |

### 9.3 Parameterized decisions

- `ota_device_control_cache_policy(verified_active_handoff)`: a verified
  active handoff makes the device handle reusable but control writes still
  require fresh characteristic handles, so it yields `uncached_only`;
  otherwise `uncached_first`.
- `ota_service_endpoint_cache_policy(allow_cached)`: `uncached_first` when
  the caller proved the endpoint identity is current, `uncached_only`
  otherwise.
