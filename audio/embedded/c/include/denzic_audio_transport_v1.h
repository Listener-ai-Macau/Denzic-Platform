#ifndef DENZIC_AUDIO_TRANSPORT_V1_H
#define DENZIC_AUDIO_TRANSPORT_V1_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "denzic_audio_transport_v1_generated.h"

#ifdef __cplusplus
extern "C" {
#endif

/*
 * Product-independent VKA1 audio stream transport engine. The core owns the
 * decisions and containers that audio/protocol/flow_control_v1.md makes
 * normative:
 *  - media-clock pacing debt (38.4 kB/s target, 10 ms ticks),
 *  - true-capacity backpressure hysteresis (pause >= 95%, resume <= 70%),
 *  - the 48-packet replay window with retain-until-notify-success semantics,
 *  - connection-epoch stale-event classification.
 *
 * Everything that touches an OS or a BLE stack stays in the product adapter:
 * timers and delays, notify submission, memory pools, queues, logging, and
 * diagnostic counters. The core never blocks, never allocates, and never
 * logs; adapters feed it facts and execute its decisions.
 */

/* --- Media-clock pacing ------------------------------------------------- */

typedef struct {
    uint32_t debt_bytes;
} denzic_audio_transport_v1_pacing_t;

void denzic_audio_transport_v1_pacing_init(
    denzic_audio_transport_v1_pacing_t *pacing);

/*
 * Accounts one successfully submitted audio packet against the pacing target
 * and returns how many pacing ticks the adapter must wait before continuing.
 * Debt is tracked in source PCM bytes; the remainder carries over so the
 * long-run rate matches DENZIC_AUDIO_TRANSPORT_V1_PACING_BYTES_PER_TICK.
 */
uint32_t denzic_audio_transport_v1_pacing_note_pcm_sent(
    denzic_audio_transport_v1_pacing_t *pacing,
    uint32_t packet_pcm_bytes);

/* --- Backpressure hysteresis -------------------------------------------- */

/* Pressure is the larger of queue and pool occupancy, in percent. */
uint32_t denzic_audio_transport_v1_pressure_percent(
    uint32_t queue_depth,
    uint32_t queue_capacity,
    uint32_t pool_in_use,
    uint32_t pool_capacity);

/*
 * Hysteresis decision from flow_control_v1.md section 3: without an active
 * session capture never stays paused; pressure >= pause% pauses; pressure
 * <= resume% resumes; between the thresholds the previous state holds.
 */
bool denzic_audio_transport_v1_backpressure_decide(
    bool pause_active,
    bool session_active,
    uint32_t pressure_percent);

/* Pool occupancy level (in buffers) at which the once-per-session warning
 * from flow_control_v1.md section 3 fires; the latch stays in the adapter. */
uint32_t denzic_audio_transport_v1_pool_warn_level(uint32_t pool_capacity);

/* --- Notify value sizing ------------------------------------------------- */

/*
 * Maps an exchanged ATT MTU to the notify value byte budget: MTU minus the
 * ATT overhead, clamped to [header_bytes, PACKET_MAX_VALUE_BYTES]. Returns 0
 * when the MTU carries no ATT payload and the adapter must keep its previous
 * value.
 */
uint16_t denzic_audio_transport_v1_packet_value_max_from_mtu(
    uint16_t mtu,
    uint16_t header_bytes);

/* --- Connection epoch and stale events ----------------------------------- */

/* Next epoch value; zero is reserved and skipped on wrap. */
uint32_t denzic_audio_transport_v1_next_epoch(uint32_t current_epoch);

/*
 * flow_control_v1.md section 2: a subscribe/MTU/notify_tx event applies only
 * when both the connection handle and the epoch captured at issue time match
 * the current link. Anything else is stale and may only be counted.
 */
bool denzic_audio_transport_v1_event_applies(
    uint16_t current_conn_handle,
    uint32_t current_epoch,
    uint16_t event_conn_handle,
    uint32_t event_epoch);

/* Disconnect events carry no epoch; only the connection handle decides. */
bool denzic_audio_transport_v1_link_event_applies(
    uint16_t current_conn_handle,
    uint16_t event_conn_handle);

/* --- Replay window -------------------------------------------------------- */

typedef struct {
    bool valid;
    uint32_t session_id;
    uint16_t sequence;
    uint16_t payload_len;
    uint16_t packet_pcm_bytes;
    uint8_t flags;
    uint8_t payload[DENZIC_AUDIO_TRANSPORT_V1_REPLAY_PAYLOAD_BYTES];
} denzic_audio_transport_v1_replay_packet_t;

typedef struct {
    denzic_audio_transport_v1_replay_packet_t packets[
        DENZIC_AUDIO_TRANSPORT_V1_REPLAY_WINDOW_PACKETS];
    uint32_t next_index;
    bool pending;
    uint32_t pending_session_id;
    bool in_progress;
} denzic_audio_transport_v1_replay_window_t;

typedef enum {
    DENZIC_AUDIO_TRANSPORT_V1_REPLAY_STORE_IGNORED = 0,
    DENZIC_AUDIO_TRANSPORT_V1_REPLAY_STORE_NEW,
    DENZIC_AUDIO_TRANSPORT_V1_REPLAY_STORE_REPLACED
} denzic_audio_transport_v1_replay_store_result_t;

void denzic_audio_transport_v1_replay_window_init(
    denzic_audio_transport_v1_replay_window_t *window);

/*
 * Retains a copy of an audio packet for replay. Storage is ignored (and the
 * caller can tell from the result) when the session id is zero, the payload
 * is missing, empty, or larger than the window payload budget, or a drain is
 * in progress. Re-retaining an existing (session, sequence) replaces the
 * previous copy in place; otherwise the oldest slot is evicted round-robin,
 * which is what bounds retention to the most recent 48 packets. Flags —
 * including the lossless Rice flag — are preserved verbatim for resend.
 */
denzic_audio_transport_v1_replay_store_result_t
denzic_audio_transport_v1_replay_store(
    denzic_audio_transport_v1_replay_window_t *window,
    uint32_t session_id,
    uint16_t sequence,
    const uint8_t *payload,
    uint16_t payload_len,
    uint16_t packet_pcm_bytes,
    uint8_t flags);

/* Drops a retained packet after notify success; returns true when found. */
bool denzic_audio_transport_v1_replay_remove(
    denzic_audio_transport_v1_replay_window_t *window,
    uint32_t session_id,
    uint16_t sequence);

/* Purges every retained packet of a session and dismisses its pending
 * replay; used on cancel and on terminal session teardown. */
void denzic_audio_transport_v1_replay_clear_session(
    denzic_audio_transport_v1_replay_window_t *window,
    uint32_t session_id);

uint32_t denzic_audio_transport_v1_replay_count_retained(
    const denzic_audio_transport_v1_replay_window_t *window,
    uint32_t session_id);

/*
 * Arms the retained window as pending replay after link loss or
 * notify-disabled with an active session. Returns true when at least one
 * packet was retained and the window is now pending; with nothing retained
 * the window stays untouched.
 */
bool denzic_audio_transport_v1_replay_mark_suspended(
    denzic_audio_transport_v1_replay_window_t *window,
    uint32_t session_id);

/*
 * Collects the pending packets of a session in ascending sequence order for
 * resend. When skip_current_sequence is set, the packet the live path is
 * about to send is left out and *skipped_current reports the omission. Does
 * not mutate window state; the adapter resends through its normal notify
 * path and reports the outcome with the drain calls below.
 */
uint32_t denzic_audio_transport_v1_replay_collect_pending(
    denzic_audio_transport_v1_replay_window_t *window,
    uint32_t session_id,
    bool skip_current_sequence,
    uint16_t current_sequence,
    const denzic_audio_transport_v1_replay_packet_t **out_packets,
    uint32_t out_capacity,
    bool *skipped_current);

/* Marks resend in progress; while set, replay_store ignores new packets. */
void denzic_audio_transport_v1_replay_begin_drain(
    denzic_audio_transport_v1_replay_window_t *window);

/* All pending packets resent: leaves drain mode and disarms the window. */
void denzic_audio_transport_v1_replay_complete_drain(
    denzic_audio_transport_v1_replay_window_t *window);

/* A resend failed: leaves drain mode but keeps the window pending so the
 * retained packets are retried through the normal notify retry path. */
void denzic_audio_transport_v1_replay_abort_drain(
    denzic_audio_transport_v1_replay_window_t *window);

/* Nothing left to resend for the pending session: disarms the window. */
void denzic_audio_transport_v1_replay_dismiss_pending(
    denzic_audio_transport_v1_replay_window_t *window);

#ifdef __cplusplus
}
#endif

#endif
