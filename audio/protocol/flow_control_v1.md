# VKA1 Flow-Control Contract v1

Machine-checkable semantics for the VKA1 audio stream (`audio_v1.json`).
This document is normative for both ends of the link: the device transport
and any host Companion that consumes the stream. Where a value already lives
in `audio_v1.json`, the JSON is the source of truth and this document only
interprets it; everything else is stated here as decision tables.

Protocol constants referenced below:

| Name | Value | Source |
| --- | --- | --- |
| PCM byte rate | 32,000 B/s (16,000 Hz × 1 ch × 16 bit) | `pcm` in `audio_v1.json` |
| `session_errors.queue_full` | 1 | `session_errors` in `audio_v1.json` |
| Lossless Rice packet flag | 0x01 | `lossless_rice.packet_flag` in `audio_v1.json` |
| Wire pacing target | 38,400 B/s | `pacing` in `audio_transport_v1.json` |
| Pacing tick | 10 ms (384 B/tick) | `pacing` in `audio_transport_v1.json` |
| Replay retained window | 48 audio packets | `replay` in `audio_transport_v1.json` |
| Fixed session duration | exactly 60 s = 1,920,000 PCM bytes | `fixed_session` in `audio_transport_v1.json` |

## 1. Media clock and steady consumption

- The producer emits PCM at exactly 32,000 B/s. Session durations are always
  `pcm_bytes / 32,000` seconds; hosts must derive timing from PCM byte counts,
  never from wall-clock notification arrival.
- The device paces notifications toward a 38,400 B/s wire target in 10 ms
  ticks (384 B per tick), tracking a byte debt against the PCM media clock
  (`packet_pcm_bytes`, i.e. source PCM, not compressed wire bytes). The 20%
  headroom above the 32,000 B/s production rate absorbs replay resends and
  raw-PCM fallback packets without growing latency.
- A host Companion must consume at a steady 32,000 B/s equivalent: buffering
  must tolerate burst delivery up to the wire pacing target without dropping,
  reordering, or re-timing packets.

## 2. Epoch filtering (stale-event discard)

Every link lifecycle transition (connect, disconnect) increments the
connection epoch. Asynchronous events carry the `(conn_handle, epoch)` pair
captured when they were issued.

| Event | conn_handle matches current link? | epoch matches current epoch? | Decision |
| --- | --- | --- | --- |
| subscribe result | yes | yes | apply notify-enabled state |
| subscribe result | no / stale | — | count as stale, discard |
| MTU exchange result | yes | yes | apply MTU-ready state |
| MTU exchange result | no / stale | — | count as stale, discard |
| NOTIFY_TX completion | yes | yes | release exactly that packet's resources |
| NOTIFY_TX completion | no / stale | — | count as stale, discard |
| disconnect | yes | — | tear down link, bump epoch |
| disconnect | no | — | count as stale, discard |

Rules:

- A stale event must never release current-session notify credits, remove
  replay packets, or alter session state.
- Stale events are counted per class (subscribe, MTU, notify_tx, disconnect)
  for diagnostics; counting is the only side effect allowed.

## 3. Backpressure

The notify queue and the audio packet pool are finite; backpressure is driven
by true capacity, not by estimates.

| Quantity | No SPIRAM | With SPIRAM |
| --- | --- | --- |
| Notify queue length | 48 | 256 |
| Audio pool buffers | 52 (48 + 4) | 264 (256 + 8) |

Decision table (evaluated on every queue/pool occupancy change while a
session streams):

| Condition | Decision |
| --- | --- |
| pressure ≥ 95% of true capacity (max of queue %, pool %) | pause PCM capture; `packet_sequence` must not advance while paused |
| pressure ≤ 70% | resume PCM capture |
| 70% < pressure < 95% | hold previous state (hysteresis) |
| pool occupancy ≥ 80% | log one pressure warning per session |
| queue push fails (queue full) | terminate session with `session_errors.queue_full` (1) |
| pool allocation fails | count `audio_pool_alloc_failed`; apply the same pressure rules |

Rules:

- Capture pauses only on true-capacity pressure; transport slowness alone is
  absorbed by pacing and the replay window.
- While paused, no new packet sequence numbers may be minted, so the terminal
  expected packet count stays exact.

## 4. Replay window

- The device retains the most recent 48 audio packets (`packet_sequence`,
  payload, `packet_pcm_bytes`, and flags — including the lossless Rice flag,
  which must be preserved verbatim on resend).
- A packet leaves the window only on notify success; retaining a 49th packet
  evicts the oldest; a re-retained sequence replaces the previous copy.
- On disconnect or notify-disabled with an active session, the retained
  window becomes pending replay.

| Link state at recovery | Decision |
| --- | --- |
| session still active | resend retained packets in ascending sequence order, then continue live packets |
| stop was requested during the outage | resend retained packets first, then deliver SESSION_STOP |
| cancel was requested | drop queue and replay window, deliver SESSION_CANCEL, no replay |

- A resend that fails stays retained and is retried through the normal notify
  retry path; failures are counted (`replay_resend_failed`).
- The export task must drain all retained audio before any terminal
  SESSION_STOP notify, so stale tail audio can never land after the terminal
  boundary.

## 5. Fixed-duration sessions and stop/tail semantics

- A fixed-duration session carries exactly 60 s of PCM: 1,920,000 bytes at
  32,000 B/s. The producer's packet plan, the queued packet count, and the
  terminal `expected_packet_count` in SESSION_STOP must agree exactly; the
  export task verifies its plan against the queued sequence count before
  sending the terminal packet.
- Stop sequence: queue SESSION_STOP → drain pending replay → send
  SESSION_STOP with the exact expected packet count. If the link is down when
  stop is requested, the stop stays pending and completes after link recovery
  using the same drain-then-terminate order.
- Packets that arrive after the stop boundary (tail audio, e.g. resent
  sequences) remain valid session data: hosts must append them to the
  session PCM and account for them separately (`post_stop_*` statistics),
  but they must not extend the ASR/finalization boundary, which closes at
  the stop boundary.
- Cancel purges queued jobs and the replay window immediately and terminates
  with SESSION_CANCEL; no tail audio is expected after a cancel.

## 6. Lossless Rice interaction

- Flow control is codec-agnostic: pacing debt, backpressure, replay, and the
  terminal packet count are all accounted in source PCM bytes
  (`packet_pcm_bytes`), whether the payload is raw PCM or a lossless Rice
  frame (`lossless_rice` in `audio_v1.json`).
- A host must inflate lossless frames before applying any byte-rate
  accounting, using the payload decoder and the `packet_pcm_bytes` header
  field; the 32,000 B/s media clock always refers to decoded PCM.
