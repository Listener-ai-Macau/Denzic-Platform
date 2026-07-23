# Audio capability: adapter boundary

The audio capability carries the VKA1 wire contract (`protocol/audio_v1.json`),
the transport flow-control contract (`protocol/audio_transport_v1.json`, with
normative semantics in `protocol/flow_control_v1.md`), the lossless Rice codec,
and the product-independent stream transport engine. This document states what
stays in the platform and what product adapters must inject.

## What the platform owns

- `embedded/c/src/denzic_audio_transport_v1.c` — the embedded transport core:
  - media-clock pacing debt (38.4 kB/s wire target, 10 ms ticks, accounted in
    source PCM bytes — codec-agnostic per flow_control_v1.md section 6);
  - true-capacity backpressure hysteresis (pause >= 95%, resume <= 70%, hold
    between; pool warning level at 80%, rounded up);
  - the 48-packet replay window: retain until notify success, round-robin
    eviction of the oldest packet, in-place replace on re-retained sequence,
    pending-replay arming on link suspension, ascending-order drain with
    skip-current, and drain outcome handling (complete / abort / dismiss);
  - connection-epoch arithmetic and stale-event classification
    (subscribe/MTU/notify_tx match `(conn_handle, epoch)`; disconnect matches
    `conn_handle` only);
  - ATT MTU to notify value-budget mapping.
  The core never blocks, allocates, logs, or touches an OS/BLE API.
- `host/rust/src/transport_v1.rs` — the host mirror of the same decisions and
  constants, plus receive-side helpers on `SessionCollector`
  (`has_active_recoverable_session`, `stop_drain_expired_finalizes`).
  Receive-side session reassembly, tail-audio (`post_stop_*`) accounting, and
  session statistics already live in `host/rust/src/lib.rs`.
- Generated constants on both ends from `audio_transport_v1.json`
  (`denzic_audio_transport_v1_generated.h`, `generated_transport.rs`).

## What product adapters own

- Timers and delays: the pacing core returns how many ticks to wait; the
  adapter sleeps (and feeds its watchdog).
- Notify submission and its retry/backoff policy (mbuf allocation, ENOMEM
  retries, TX timeouts); on success the adapter calls `replay_remove`, on
  link loss `replay_mark_suspended`.
- Memory pools and queues: the adapter measures queue depth and pool
  occupancy and feeds them to `pressure_percent` / `backpressure_decide`;
  pausing PCM capture is the adapter's job.
- Session statistics and diagnostic counters (stale-event counts, replay
  store/replace/remove/resent counters, pool high water, the once-per-session
  pressure-warning latch).
- Packet planning and codec selection: chunking PCM into notify-sized
  payloads and running the lossless Rice codec stays with the adapter, which
  already delegates the codec itself to `denzic_audio_lossless_v1`.
- Host capture-loop deadlines (idle timeout, stop-drain rolling window,
  link-recovery window): the platform decides *what* the outcome of an
  expired window is; the adapter owns the clocks and the error wording.

## Wiring

- Firmware compiles `audio/embedded/c/src/denzic_audio_transport_v1.c` from
  the pinned submodule and keeps its constants pinned to the generated header
  with `_Static_assert`, matching the lossless-codec pattern.
- Hosts depend on the `denzic-audio-v1-core` crate by path; transport
  constants come from `transport_v1` re-exports, never re-hardcoded.

`tools/verify.ps1` regenerates both ends (`generate_audio_transport_v1.py
--check`) and runs the C core's ctest suite and the host crate's unit tests.
