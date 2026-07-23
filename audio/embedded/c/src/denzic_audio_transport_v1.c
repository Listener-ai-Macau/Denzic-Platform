#include "denzic_audio_transport_v1.h"

#include <string.h>

/* --- Media-clock pacing ------------------------------------------------- */

void denzic_audio_transport_v1_pacing_init(
    denzic_audio_transport_v1_pacing_t *pacing)
{
    if (pacing == NULL) {
        return;
    }
    pacing->debt_bytes = 0;
}

uint32_t denzic_audio_transport_v1_pacing_note_pcm_sent(
    denzic_audio_transport_v1_pacing_t *pacing,
    uint32_t packet_pcm_bytes)
{
    if (pacing == NULL) {
        return 0;
    }
    pacing->debt_bytes += packet_pcm_bytes;
    uint32_t delay_ticks =
        pacing->debt_bytes / DENZIC_AUDIO_TRANSPORT_V1_PACING_BYTES_PER_TICK;
    pacing->debt_bytes %= DENZIC_AUDIO_TRANSPORT_V1_PACING_BYTES_PER_TICK;
    return delay_ticks;
}

/* --- Backpressure hysteresis -------------------------------------------- */

uint32_t denzic_audio_transport_v1_pressure_percent(
    uint32_t queue_depth,
    uint32_t queue_capacity,
    uint32_t pool_in_use,
    uint32_t pool_capacity)
{
    uint32_t queue_percent = 0;
    uint32_t pool_percent = 0;

    if (queue_capacity > 0) {
        queue_percent = (queue_depth * 100u) / queue_capacity;
    }
    if (pool_capacity > 0) {
        pool_percent = (pool_in_use * 100u) / pool_capacity;
    }

    return queue_percent > pool_percent ? queue_percent : pool_percent;
}

bool denzic_audio_transport_v1_backpressure_decide(
    bool pause_active,
    bool session_active,
    uint32_t pressure_percent)
{
    if (!session_active) {
        return false;
    }
    if (pressure_percent >= DENZIC_AUDIO_TRANSPORT_V1_BACKPRESSURE_PAUSE_PERCENT) {
        return true;
    }
    if (pressure_percent <= DENZIC_AUDIO_TRANSPORT_V1_BACKPRESSURE_RESUME_PERCENT) {
        return false;
    }
    return pause_active;
}

uint32_t denzic_audio_transport_v1_pool_warn_level(uint32_t pool_capacity)
{
    return (pool_capacity * DENZIC_AUDIO_TRANSPORT_V1_AUDIO_POOL_WARN_PERCENT + 99u) / 100u;
}

/* --- Notify value sizing ------------------------------------------------- */

uint16_t denzic_audio_transport_v1_packet_value_max_from_mtu(
    uint16_t mtu,
    uint16_t header_bytes)
{
    if (mtu <= DENZIC_AUDIO_TRANSPORT_V1_PACKET_MTU_ATT_OVERHEAD_BYTES) {
        return 0;
    }

    uint16_t value_max =
        (uint16_t)(mtu - DENZIC_AUDIO_TRANSPORT_V1_PACKET_MTU_ATT_OVERHEAD_BYTES);
    if (value_max > DENZIC_AUDIO_TRANSPORT_V1_PACKET_MAX_VALUE_BYTES) {
        value_max = DENZIC_AUDIO_TRANSPORT_V1_PACKET_MAX_VALUE_BYTES;
    }
    if (value_max < header_bytes) {
        value_max = header_bytes;
    }
    return value_max;
}

/* --- Connection epoch and stale events ----------------------------------- */

uint32_t denzic_audio_transport_v1_next_epoch(uint32_t current_epoch)
{
    uint32_t next_epoch = current_epoch + 1u;
    return next_epoch == 0 ? 1u : next_epoch;
}

bool denzic_audio_transport_v1_event_applies(
    uint16_t current_conn_handle,
    uint32_t current_epoch,
    uint16_t event_conn_handle,
    uint32_t event_epoch)
{
    return event_conn_handle == current_conn_handle && event_epoch == current_epoch;
}

bool denzic_audio_transport_v1_link_event_applies(
    uint16_t current_conn_handle,
    uint16_t event_conn_handle)
{
    return event_conn_handle == current_conn_handle;
}

/* --- Replay window -------------------------------------------------------- */

void denzic_audio_transport_v1_replay_window_init(
    denzic_audio_transport_v1_replay_window_t *window)
{
    if (window == NULL) {
        return;
    }
    memset(window, 0, sizeof(*window));
}

denzic_audio_transport_v1_replay_store_result_t
denzic_audio_transport_v1_replay_store(
    denzic_audio_transport_v1_replay_window_t *window,
    uint32_t session_id,
    uint16_t sequence,
    const uint8_t *payload,
    uint16_t payload_len,
    uint16_t packet_pcm_bytes,
    uint8_t flags)
{
    if (window == NULL || session_id == 0 || payload == NULL || payload_len == 0 ||
        payload_len > DENZIC_AUDIO_TRANSPORT_V1_REPLAY_PAYLOAD_BYTES ||
        window->in_progress) {
        return DENZIC_AUDIO_TRANSPORT_V1_REPLAY_STORE_IGNORED;
    }

    for (uint32_t i = 0; i < DENZIC_AUDIO_TRANSPORT_V1_REPLAY_WINDOW_PACKETS; ++i) {
        denzic_audio_transport_v1_replay_packet_t *existing = &window->packets[i];
        if (existing->valid && existing->session_id == session_id &&
            existing->sequence == sequence) {
            existing->payload_len = payload_len;
            existing->packet_pcm_bytes = packet_pcm_bytes;
            existing->flags = flags;
            memcpy(existing->payload, payload, payload_len);
            return DENZIC_AUDIO_TRANSPORT_V1_REPLAY_STORE_REPLACED;
        }
    }

    denzic_audio_transport_v1_replay_packet_t *slot =
        &window->packets[window->next_index % DENZIC_AUDIO_TRANSPORT_V1_REPLAY_WINDOW_PACKETS];
    memset(slot, 0, sizeof(*slot));
    slot->valid = true;
    slot->session_id = session_id;
    slot->sequence = sequence;
    slot->payload_len = payload_len;
    slot->packet_pcm_bytes = packet_pcm_bytes;
    slot->flags = flags;
    memcpy(slot->payload, payload, payload_len);
    window->next_index++;
    return DENZIC_AUDIO_TRANSPORT_V1_REPLAY_STORE_NEW;
}

bool denzic_audio_transport_v1_replay_remove(
    denzic_audio_transport_v1_replay_window_t *window,
    uint32_t session_id,
    uint16_t sequence)
{
    if (window == NULL || session_id == 0) {
        return false;
    }

    for (uint32_t i = 0; i < DENZIC_AUDIO_TRANSPORT_V1_REPLAY_WINDOW_PACKETS; ++i) {
        denzic_audio_transport_v1_replay_packet_t *packet = &window->packets[i];
        if (packet->valid && packet->session_id == session_id &&
            packet->sequence == sequence) {
            memset(packet, 0, sizeof(*packet));
            return true;
        }
    }
    return false;
}

void denzic_audio_transport_v1_replay_clear_session(
    denzic_audio_transport_v1_replay_window_t *window,
    uint32_t session_id)
{
    if (window == NULL || session_id == 0) {
        return;
    }

    for (uint32_t i = 0; i < DENZIC_AUDIO_TRANSPORT_V1_REPLAY_WINDOW_PACKETS; ++i) {
        if (window->packets[i].valid && window->packets[i].session_id == session_id) {
            memset(&window->packets[i], 0, sizeof(window->packets[i]));
        }
    }
    if (window->pending && window->pending_session_id == session_id) {
        window->pending = false;
        window->pending_session_id = 0;
    }
}

uint32_t denzic_audio_transport_v1_replay_count_retained(
    const denzic_audio_transport_v1_replay_window_t *window,
    uint32_t session_id)
{
    if (window == NULL) {
        return 0;
    }

    uint32_t retained = 0;
    for (uint32_t i = 0; i < DENZIC_AUDIO_TRANSPORT_V1_REPLAY_WINDOW_PACKETS; ++i) {
        if (window->packets[i].valid && window->packets[i].session_id == session_id) {
            retained++;
        }
    }
    return retained;
}

bool denzic_audio_transport_v1_replay_mark_suspended(
    denzic_audio_transport_v1_replay_window_t *window,
    uint32_t session_id)
{
    if (window == NULL || session_id == 0) {
        return false;
    }

    if (denzic_audio_transport_v1_replay_count_retained(window, session_id) == 0) {
        return false;
    }

    window->pending = true;
    window->pending_session_id = session_id;
    return true;
}

uint32_t denzic_audio_transport_v1_replay_collect_pending(
    denzic_audio_transport_v1_replay_window_t *window,
    uint32_t session_id,
    bool skip_current_sequence,
    uint16_t current_sequence,
    const denzic_audio_transport_v1_replay_packet_t **out_packets,
    uint32_t out_capacity,
    bool *skipped_current)
{
    if (skipped_current != NULL) {
        *skipped_current = false;
    }
    if (window == NULL || out_packets == NULL || out_capacity == 0 ||
        window->in_progress || !window->pending ||
        window->pending_session_id != session_id) {
        return 0;
    }

    uint32_t count = 0;
    bool skipped = false;
    for (uint32_t i = 0; i < DENZIC_AUDIO_TRANSPORT_V1_REPLAY_WINDOW_PACKETS; ++i) {
        denzic_audio_transport_v1_replay_packet_t *packet = &window->packets[i];
        if (!packet->valid || packet->session_id != session_id) {
            continue;
        }
        if (skip_current_sequence && packet->sequence == current_sequence) {
            skipped = true;
            continue;
        }
        if (count < out_capacity) {
            out_packets[count++] = packet;
        }
    }
    if (skipped_current != NULL) {
        *skipped_current = skipped;
    }

    /* Ascending sequence order; the window is small and nearly sorted. */
    for (uint32_t i = 1; i < count; ++i) {
        const denzic_audio_transport_v1_replay_packet_t *entry = out_packets[i];
        uint32_t j = i;
        while (j > 0 && out_packets[j - 1]->sequence > entry->sequence) {
            out_packets[j] = out_packets[j - 1];
            --j;
        }
        out_packets[j] = entry;
    }
    return count;
}

void denzic_audio_transport_v1_replay_begin_drain(
    denzic_audio_transport_v1_replay_window_t *window)
{
    if (window == NULL) {
        return;
    }
    window->in_progress = true;
}

void denzic_audio_transport_v1_replay_complete_drain(
    denzic_audio_transport_v1_replay_window_t *window)
{
    if (window == NULL) {
        return;
    }
    window->in_progress = false;
    window->pending = false;
    window->pending_session_id = 0;
}

void denzic_audio_transport_v1_replay_abort_drain(
    denzic_audio_transport_v1_replay_window_t *window)
{
    if (window == NULL) {
        return;
    }
    window->in_progress = false;
}

void denzic_audio_transport_v1_replay_dismiss_pending(
    denzic_audio_transport_v1_replay_window_t *window)
{
    if (window == NULL) {
        return;
    }
    window->pending = false;
    window->pending_session_id = 0;
}
