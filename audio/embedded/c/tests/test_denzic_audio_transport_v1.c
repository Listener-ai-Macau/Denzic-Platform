#include "denzic_audio_transport_v1.h"

#include <stdio.h>
#include <stdlib.h>
#include <string.h>

/* Release builds define NDEBUG, so these tests check explicitly instead of
 * relying on CHECK(). */
#define CHECK(condition)                                                      \
    do {                                                                      \
        if (!(condition)) {                                                   \
            fprintf(stderr, "CHECK failed at %s:%d: %s\n", __FILE__,          \
                    __LINE__, #condition);                                    \
            exit(1);                                                          \
        }                                                                     \
    } while (0)

#define TEST_SESSION_ID 7u

static void fill_payload(uint8_t *payload, uint16_t payload_len, uint8_t seed)
{
    for (uint16_t i = 0; i < payload_len; ++i) {
        payload[i] = (uint8_t)(seed + i);
    }
}

static void test_pacing_accumulates_debt_across_packets(void)
{
    denzic_audio_transport_v1_pacing_t pacing;
    denzic_audio_transport_v1_pacing_init(&pacing);

    /* 480-byte packets against a 384-bytes-per-tick target cycle 1/1/1/2
     * ticks, averaging 38.4 kB/s on a 32 kB/s media clock. */
    CHECK(denzic_audio_transport_v1_pacing_note_pcm_sent(&pacing, 480u) == 1u);
    CHECK(denzic_audio_transport_v1_pacing_note_pcm_sent(&pacing, 480u) == 1u);
    CHECK(denzic_audio_transport_v1_pacing_note_pcm_sent(&pacing, 480u) == 1u);
    CHECK(denzic_audio_transport_v1_pacing_note_pcm_sent(&pacing, 480u) == 2u);

    denzic_audio_transport_v1_pacing_init(&pacing);
    CHECK(denzic_audio_transport_v1_pacing_note_pcm_sent(&pacing, 100u) == 0u);
    CHECK(denzic_audio_transport_v1_pacing_note_pcm_sent(&pacing, 284u) == 1u);
    CHECK(denzic_audio_transport_v1_pacing_note_pcm_sent(&pacing, 0u) == 0u);

    /* One second of PCM paces to floor(32000/384) = 83 ticks; the 128-byte
     * remainder carries into the next second, matching the 38.4 kB/s target. */
    denzic_audio_transport_v1_pacing_init(&pacing);
    uint32_t total_ticks = 0;
    for (uint32_t i = 0; i < 100u; ++i) {
        total_ticks +=
            denzic_audio_transport_v1_pacing_note_pcm_sent(&pacing, 320u);
    }
    CHECK(total_ticks == 83u);
    CHECK(pacing.debt_bytes == 128u);
}

static void test_pressure_percent_uses_true_capacity_max(void)
{
    CHECK(denzic_audio_transport_v1_pressure_percent(24u, 48u, 0u, 52u) == 50u);
    CHECK(denzic_audio_transport_v1_pressure_percent(0u, 48u, 26u, 52u) == 50u);
    CHECK(denzic_audio_transport_v1_pressure_percent(47u, 48u, 10u, 52u) == 97u);
    CHECK(denzic_audio_transport_v1_pressure_percent(0u, 0u, 0u, 0u) == 0u);
    CHECK(denzic_audio_transport_v1_pressure_percent(10u, 0u, 0u, 0u) == 0u);
}

static void test_backpressure_hysteresis(void)
{
    /* No active session: capture never stays paused. */
    CHECK(!denzic_audio_transport_v1_backpressure_decide(true, false, 100u));
    CHECK(!denzic_audio_transport_v1_backpressure_decide(false, false, 0u));

    /* Pause at the 95% threshold, hold inside the band, resume at 70%. */
    bool pause = denzic_audio_transport_v1_backpressure_decide(false, true, 95u);
    CHECK(pause);
    pause = denzic_audio_transport_v1_backpressure_decide(pause, true, 80u);
    CHECK(pause);
    pause = denzic_audio_transport_v1_backpressure_decide(pause, true, 71u);
    CHECK(pause);
    pause = denzic_audio_transport_v1_backpressure_decide(pause, true, 70u);
    CHECK(!pause);
    pause = denzic_audio_transport_v1_backpressure_decide(pause, true, 94u);
    CHECK(!pause);
    pause = denzic_audio_transport_v1_backpressure_decide(pause, true, 100u);
    CHECK(pause);
}

static void test_pool_warn_level_rounds_up(void)
{
    CHECK(denzic_audio_transport_v1_pool_warn_level(52u) == 42u);
    CHECK(denzic_audio_transport_v1_pool_warn_level(264u) == 212u);
    CHECK(denzic_audio_transport_v1_pool_warn_level(0u) == 0u);
}

static void test_packet_value_max_from_mtu(void)
{
    CHECK(denzic_audio_transport_v1_packet_value_max_from_mtu(0u, 20u) == 0u);
    CHECK(denzic_audio_transport_v1_packet_value_max_from_mtu(3u, 20u) == 0u);
    CHECK(denzic_audio_transport_v1_packet_value_max_from_mtu(4u, 20u) == 20u);
    CHECK(denzic_audio_transport_v1_packet_value_max_from_mtu(247u, 20u) == 244u);
    CHECK(denzic_audio_transport_v1_packet_value_max_from_mtu(517u, 20u) == 500u);
    CHECK(denzic_audio_transport_v1_packet_value_max_from_mtu(600u, 20u) == 500u);
}

static void test_epoch_advance_skips_zero(void)
{
    CHECK(denzic_audio_transport_v1_next_epoch(0u) == 1u);
    CHECK(denzic_audio_transport_v1_next_epoch(41u) == 42u);
    CHECK(denzic_audio_transport_v1_next_epoch(0xFFFFFFFFu) == 1u);
}

static void test_stale_event_classification(void)
{
    /* Subscribe/MTU/notify_tx: conn and epoch must both match. */
    CHECK(denzic_audio_transport_v1_event_applies(1u, 9u, 1u, 9u));
    CHECK(!denzic_audio_transport_v1_event_applies(1u, 9u, 2u, 9u));
    CHECK(!denzic_audio_transport_v1_event_applies(1u, 9u, 1u, 8u));
    CHECK(!denzic_audio_transport_v1_event_applies(1u, 9u, 0xFFFFu, 9u));

    /* Disconnect: only the connection handle decides. */
    CHECK(denzic_audio_transport_v1_link_event_applies(1u, 1u));
    CHECK(!denzic_audio_transport_v1_link_event_applies(1u, 2u));
}

static void test_replay_store_replace_and_eviction(void)
{
    denzic_audio_transport_v1_replay_window_t window;
    denzic_audio_transport_v1_replay_window_init(&window);

    uint8_t payload[DENZIC_AUDIO_TRANSPORT_V1_REPLAY_PAYLOAD_BYTES];
    fill_payload(payload, sizeof(payload), 3u);

    /* Invalid stores are ignored and observable as such. */
    CHECK(denzic_audio_transport_v1_replay_store(
               &window, 0u, 0u, payload, 10u, 10u, 0u) ==
           DENZIC_AUDIO_TRANSPORT_V1_REPLAY_STORE_IGNORED);
    CHECK(denzic_audio_transport_v1_replay_store(
               &window, TEST_SESSION_ID, 0u, NULL, 10u, 10u, 0u) ==
           DENZIC_AUDIO_TRANSPORT_V1_REPLAY_STORE_IGNORED);
    CHECK(denzic_audio_transport_v1_replay_store(
               &window, TEST_SESSION_ID, 0u, payload, 0u, 10u, 0u) ==
           DENZIC_AUDIO_TRANSPORT_V1_REPLAY_STORE_IGNORED);
    CHECK(denzic_audio_transport_v1_replay_store(
               &window, TEST_SESSION_ID, 0u, payload,
               (uint16_t)(DENZIC_AUDIO_TRANSPORT_V1_REPLAY_PAYLOAD_BYTES + 1u),
               10u, 0u) == DENZIC_AUDIO_TRANSPORT_V1_REPLAY_STORE_IGNORED);

    CHECK(denzic_audio_transport_v1_replay_store(
               &window, TEST_SESSION_ID, 5u, payload, 100u, 96u, 0x01u) ==
           DENZIC_AUDIO_TRANSPORT_V1_REPLAY_STORE_NEW);
    CHECK(denzic_audio_transport_v1_replay_count_retained(&window, TEST_SESSION_ID) == 1u);

    /* Re-retaining the same sequence replaces the copy in place, preserving
     * the lossless Rice flag verbatim for resend. */
    fill_payload(payload, 40u, 9u);
    CHECK(denzic_audio_transport_v1_replay_store(
               &window, TEST_SESSION_ID, 5u, payload, 40u, 40u, 0x01u) ==
           DENZIC_AUDIO_TRANSPORT_V1_REPLAY_STORE_REPLACED);
    CHECK(denzic_audio_transport_v1_replay_count_retained(&window, TEST_SESSION_ID) == 1u);
    CHECK(window.packets[0].payload_len == 40u);
    CHECK(window.packets[0].flags == 0x01u);
    CHECK(memcmp(window.packets[0].payload, payload, 40u) == 0);

    /* Retaining a 49th packet evicts the oldest. */
    for (uint32_t sequence = 100u; sequence < 148u; ++sequence) {
        CHECK(denzic_audio_transport_v1_replay_store(
                   &window, TEST_SESSION_ID, (uint16_t)sequence,
                   payload, 10u, 10u, 0u) ==
               DENZIC_AUDIO_TRANSPORT_V1_REPLAY_STORE_NEW);
    }
    CHECK(denzic_audio_transport_v1_replay_count_retained(&window, TEST_SESSION_ID) ==
           DENZIC_AUDIO_TRANSPORT_V1_REPLAY_WINDOW_PACKETS);
    CHECK(!denzic_audio_transport_v1_replay_remove(&window, TEST_SESSION_ID, 5u));
    CHECK(denzic_audio_transport_v1_replay_remove(&window, TEST_SESSION_ID, 147u));
}

static void test_replay_remove_and_clear_session(void)
{
    denzic_audio_transport_v1_replay_window_t window;
    denzic_audio_transport_v1_replay_window_init(&window);

    uint8_t payload[16];
    fill_payload(payload, sizeof(payload), 1u);
    CHECK(denzic_audio_transport_v1_replay_store(
               &window, 1u, 0u, payload, 16u, 16u, 0u) ==
           DENZIC_AUDIO_TRANSPORT_V1_REPLAY_STORE_NEW);
    CHECK(denzic_audio_transport_v1_replay_store(
               &window, 2u, 0u, payload, 16u, 16u, 0u) ==
           DENZIC_AUDIO_TRANSPORT_V1_REPLAY_STORE_NEW);

    /* Notify success removes exactly that packet. */
    CHECK(denzic_audio_transport_v1_replay_remove(&window, 1u, 0u));
    CHECK(!denzic_audio_transport_v1_replay_remove(&window, 1u, 0u));
    CHECK(denzic_audio_transport_v1_replay_count_retained(&window, 1u) == 0u);
    CHECK(denzic_audio_transport_v1_replay_count_retained(&window, 2u) == 1u);

    CHECK(denzic_audio_transport_v1_replay_mark_suspended(&window, 2u));
    denzic_audio_transport_v1_replay_clear_session(&window, 2u);
    CHECK(denzic_audio_transport_v1_replay_count_retained(&window, 2u) == 0u);
    CHECK(!window.pending);
    CHECK(window.pending_session_id == 0u);
}

static void test_replay_mark_suspended_requires_retained_packets(void)
{
    denzic_audio_transport_v1_replay_window_t window;
    denzic_audio_transport_v1_replay_window_init(&window);

    CHECK(!denzic_audio_transport_v1_replay_mark_suspended(&window, TEST_SESSION_ID));
    CHECK(!window.pending);

    uint8_t payload[8];
    fill_payload(payload, sizeof(payload), 2u);
    CHECK(denzic_audio_transport_v1_replay_store(
               &window, TEST_SESSION_ID, 0u, payload, 8u, 8u, 0u) ==
           DENZIC_AUDIO_TRANSPORT_V1_REPLAY_STORE_NEW);
    CHECK(denzic_audio_transport_v1_replay_mark_suspended(&window, TEST_SESSION_ID));
    CHECK(window.pending);
    CHECK(window.pending_session_id == TEST_SESSION_ID);
}

static void test_replay_collect_ascending_with_skip_current(void)
{
    denzic_audio_transport_v1_replay_window_t window;
    denzic_audio_transport_v1_replay_window_init(&window);

    uint8_t payload[8];
    fill_payload(payload, sizeof(payload), 4u);
    /* Store out of order: 12, 10, 11. */
    CHECK(denzic_audio_transport_v1_replay_store(
               &window, TEST_SESSION_ID, 12u, payload, 8u, 8u, 0u) ==
           DENZIC_AUDIO_TRANSPORT_V1_REPLAY_STORE_NEW);
    CHECK(denzic_audio_transport_v1_replay_store(
               &window, TEST_SESSION_ID, 10u, payload, 8u, 8u, 0u) ==
           DENZIC_AUDIO_TRANSPORT_V1_REPLAY_STORE_NEW);
    CHECK(denzic_audio_transport_v1_replay_store(
               &window, TEST_SESSION_ID, 11u, payload, 8u, 8u, 0u) ==
           DENZIC_AUDIO_TRANSPORT_V1_REPLAY_STORE_NEW);
    /* A foreign session packet never joins the drain. */
    CHECK(denzic_audio_transport_v1_replay_store(
               &window, 99u, 1u, payload, 8u, 8u, 0u) ==
           DENZIC_AUDIO_TRANSPORT_V1_REPLAY_STORE_NEW);

    /* Nothing pending yet: collect yields nothing. */
    const denzic_audio_transport_v1_replay_packet_t *drain[4];
    bool skipped = false;
    CHECK(denzic_audio_transport_v1_replay_collect_pending(
               &window, TEST_SESSION_ID, false, 0u, drain, 4u, &skipped) == 0u);

    CHECK(denzic_audio_transport_v1_replay_mark_suspended(&window, TEST_SESSION_ID));
    memset(drain, 0, sizeof(drain));
    CHECK(denzic_audio_transport_v1_replay_collect_pending(
               &window, TEST_SESSION_ID, false, 0u, drain, 4u, &skipped) == 3u);
    CHECK(!skipped);
    CHECK(drain[0]->sequence == 10u);
    CHECK(drain[1]->sequence == 11u);
    CHECK(drain[2]->sequence == 12u);

    /* The live packet is skipped once but stays retained. */
    memset(drain, 0, sizeof(drain));
    CHECK(denzic_audio_transport_v1_replay_collect_pending(
               &window, TEST_SESSION_ID, true, 11u, drain, 4u, &skipped) == 2u);
    CHECK(skipped);
    CHECK(drain[0]->sequence == 10u);
    CHECK(drain[1]->sequence == 12u);
    CHECK(denzic_audio_transport_v1_replay_count_retained(&window, TEST_SESSION_ID) == 3u);
}

static void test_replay_drain_outcomes(void)
{
    denzic_audio_transport_v1_replay_window_t window;
    denzic_audio_transport_v1_replay_window_init(&window);

    uint8_t payload[8];
    fill_payload(payload, sizeof(payload), 5u);
    CHECK(denzic_audio_transport_v1_replay_store(
               &window, TEST_SESSION_ID, 0u, payload, 8u, 8u, 0u) ==
           DENZIC_AUDIO_TRANSPORT_V1_REPLAY_STORE_NEW);
    CHECK(denzic_audio_transport_v1_replay_mark_suspended(&window, TEST_SESSION_ID));

    /* While a drain runs, stores are ignored and collects yield nothing. */
    denzic_audio_transport_v1_replay_begin_drain(&window);
    CHECK(denzic_audio_transport_v1_replay_store(
               &window, TEST_SESSION_ID, 1u, payload, 8u, 8u, 0u) ==
           DENZIC_AUDIO_TRANSPORT_V1_REPLAY_STORE_IGNORED);
    const denzic_audio_transport_v1_replay_packet_t *drain[2];
    CHECK(denzic_audio_transport_v1_replay_collect_pending(
               &window, TEST_SESSION_ID, false, 0u, drain, 2u, NULL) == 0u);

    /* A failed resend keeps the window pending for the notify retry path. */
    denzic_audio_transport_v1_replay_abort_drain(&window);
    CHECK(!window.in_progress);
    CHECK(window.pending);
    CHECK(window.pending_session_id == TEST_SESSION_ID);

    /* A completed drain disarms the window; retained packets leave only via
     * notify success (replay_remove). */
    denzic_audio_transport_v1_replay_begin_drain(&window);
    denzic_audio_transport_v1_replay_complete_drain(&window);
    CHECK(!window.in_progress);
    CHECK(!window.pending);
    CHECK(window.pending_session_id == 0u);
    CHECK(denzic_audio_transport_v1_replay_count_retained(&window, TEST_SESSION_ID) == 1u);

    /* Dismiss handles the pending-but-nothing-left case. */
    CHECK(denzic_audio_transport_v1_replay_mark_suspended(&window, TEST_SESSION_ID));
    denzic_audio_transport_v1_replay_dismiss_pending(&window);
    CHECK(!window.pending);
    CHECK(window.pending_session_id == 0u);
}

int main(void)
{
    test_pacing_accumulates_debt_across_packets();
    test_pressure_percent_uses_true_capacity_max();
    test_backpressure_hysteresis();
    test_pool_warn_level_rounds_up();
    test_packet_value_max_from_mtu();
    test_epoch_advance_skips_zero();
    test_stale_event_classification();
    test_replay_store_replace_and_eviction();
    test_replay_remove_and_clear_session();
    test_replay_mark_suspended_requires_retained_packets();
    test_replay_collect_ascending_with_skip_current();
    test_replay_drain_outcomes();
    return 0;
}
