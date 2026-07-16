# Device Control v1 Adapter Contract

`denzic_device_control_v1` owns the product-independent facts of a device
connection and control transaction. It does not own radio APIs, HID, LED
effects, input gestures, device-specific command values, or UI.

## Interaction Model

The contract follows the small, transport-neutral interaction split used by
the W3C Web of Things: a device exposes readable state, runs time-bounded
actions, and emits asynchronous state changes. The Platform representation is
deliberately not a WoT wire format; product adapters choose BLE, USB, Wi-Fi,
serial, or another transport.

- `ReadCapabilities` records a capability revision and opaque capability mask.
- `ReadSetting` and `WriteSetting` represent configuration transactions.
- `InvokeCommand` represents a product-defined command without leaking its
  product-specific meaning into the shared core.
- `LifecycleState` and `OwnershipState` are asynchronous facts supplied by
  the adapter.

## Transaction Rules

- A request has a non-zero `operation_id`, `idempotency_key`, target id, and
  deadline. Only a decision with `execute=true` may cause the adapter to send
  a transport request.
- A duplicate active request returns `accepted` with `replayed=true` and
  `execute=false`. A duplicate terminal request returns the stored terminal
  result with `replayed=true`. The embedded core keeps a four-entry bounded
  terminal history; adapters needing durable recovery keep the same operation
  identity in their own durable executor ledger.
- A non-idempotent operation with an unknown outcome must not be resent under
  a new identity. It must become a classified terminal failure, query an
  authoritative readback, or wait for an explicit new user action.
- `Cancel`, deadline expiry, transport loss, external ownership, and a manual
  unpair all make an active operation terminal. A terminal recovery capsule
  must be removed by the adapter rather than retrying forever.
- A `WriteSetting` cannot complete with `succeeded` directly. The adapter must
  first report its acknowledgement revision and then matching-or-newer setting
  readback. This prevents a local write from being treated as applied merely
  because a transport write returned.

## Ownership And Recovery

Ownership is a security and host-policy fact, never an inference from a BLE
random address. `manual_unpaired` rejects automatic connection attempts, so a
Windows manual deletion remains passive. `external` moves the lifecycle to
`owned_elsewhere`, terminates active work with `failed/ownership`, and rejects
automatic reclaim. A product may perform an automatic recovery only after it
has its own explicit local authorization evidence.

For Listener, the adapter maps the accepted local Type-owned EC11 double-click
path to `automatic=true` with explicit recovery authorization. It maps manual
Windows removal to `manual_unpaired`, and a successful other-host secure
binding to `external`. The Platform core therefore preserves the accepted
behavior without importing EC11, PairAsync, or Windows APIs.

## Observability Boundary

This module gives operations and state transitions their stable identity. The
separate `denzic_observability_v1` envelope carries correlation IDs, timing,
source attribution, and emitted evidence. Device control must not delay or
replace those events.

## Research Basis

- W3C Thing Description 2.0 defines the portable Property, Action, and Event
  interaction model: https://www.w3.org/TR/wot-thing-description-2.0/
- RFC 9110 says a client must not automatically retry a non-idempotent request
  unless it knows the original semantics are idempotent or were not applied:
  https://www.rfc-editor.org/rfc/rfc9110.html
- Bluetooth Core Security Manager defines pairing, encryption, distributed
  keys, and identity as security lifecycle data, not address heuristics:
  https://www.bluetooth.com/wp-content/uploads/Files/Specification/HTML/Core_v6.3/out/en/host/security-manager-specification.html
- Windows pairing is an explicit asynchronous `PairAsync`/`UnpairAsync`
  operation, not a fabricated local connection state:
  https://learn.microsoft.com/en-us/windows/apps/develop/devices-sensors/pair-devices
