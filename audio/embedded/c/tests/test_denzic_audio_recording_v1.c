#include "denzic_audio_recording_v1.h"

#include <assert.h>
#include <string.h>

static void test_listener_style_toggle_session(void)
{
    denzic_audio_recording_context_v1_t context;
    denzic_audio_recording_init_v1(&context, NULL);

    assert(denzic_audio_recording_dispatch_v1(
               &context, DENZIC_AUDIO_RECORDING_EVENT_TOGGLE_REQUEST,
               DENZIC_AUDIO_RECORDING_SOURCE_USER_TOUCH) ==
           DENZIC_AUDIO_RECORDING_RESULT_OK);
    assert(context.state == DENZIC_AUDIO_RECORDING_STATE_RECORDING);
    assert(context.active_source == DENZIC_AUDIO_RECORDING_SOURCE_USER_TOUCH);
    assert(context.metadata.recording_id == 1u);

    const denzic_audio_frame_batch_v1_t batch = {
        .frame_count = 2u,
        .samples_per_frame = 160u,
        .bytes_per_frame = 320u,
        .duration_ms = 20u,
        .dropped_frames = 1u,
        .crc32 = 0x12345678u,
    };
    assert(denzic_audio_recording_feed_v1(&context, &batch) ==
           DENZIC_AUDIO_RECORDING_RESULT_OK);
    assert(context.metadata.frame_count == 2u);
    assert(context.metadata.total_length == 640u);
    assert(context.metadata.duration_ms == 20u);
    assert(context.metadata.dropped_frames == 1u);

    assert(denzic_audio_recording_dispatch_v1(
               &context, DENZIC_AUDIO_RECORDING_EVENT_TOGGLE_REQUEST,
               DENZIC_AUDIO_RECORDING_SOURCE_USER_TOUCH) ==
           DENZIC_AUDIO_RECORDING_RESULT_OK);
    assert(context.state == DENZIC_AUDIO_RECORDING_STATE_STOPPING);
    assert(denzic_audio_recording_dispatch_v1(
               &context, DENZIC_AUDIO_RECORDING_EVENT_SESSION_FINISHED,
               DENZIC_AUDIO_RECORDING_SOURCE_UNKNOWN) ==
           DENZIC_AUDIO_RECORDING_RESULT_OK);
    assert(context.state == DENZIC_AUDIO_RECORDING_STATE_TRANSFER_READY);
    assert(context.metadata.byte_offset_ready == 640u);
    assert(denzic_audio_recording_dispatch_v1(
               &context, DENZIC_AUDIO_RECORDING_EVENT_TRANSFER_BEGIN,
               DENZIC_AUDIO_RECORDING_SOURCE_HOST_CONTROL) ==
           DENZIC_AUDIO_RECORDING_RESULT_OK);
    assert(denzic_audio_recording_dispatch_v1(
               &context, DENZIC_AUDIO_RECORDING_EVENT_TRANSFER_FINISHED,
               DENZIC_AUDIO_RECORDING_SOURCE_HOST_CONTROL) ==
           DENZIC_AUDIO_RECORDING_RESULT_OK);
    assert(context.metadata.sync_state ==
           DENZIC_AUDIO_RECORDING_SYNC_TRANSFERRED);
    assert(denzic_audio_recording_dispatch_v1(
               &context, DENZIC_AUDIO_RECORDING_EVENT_RECOVERY_COMPLETE,
               DENZIC_AUDIO_RECORDING_SOURCE_UNKNOWN) ==
           DENZIC_AUDIO_RECORDING_RESULT_OK);
    assert(context.state == DENZIC_AUDIO_RECORDING_STATE_IDLE);
}

static void test_pending_capture_and_cancel(void)
{
    denzic_audio_recording_context_v1_t context;
    denzic_audio_recording_init_v1(&context, NULL);
    denzic_audio_recording_set_capture_ready_v1(&context, false);

    assert(denzic_audio_recording_dispatch_v1(
               &context, DENZIC_AUDIO_RECORDING_EVENT_START_REQUEST,
               DENZIC_AUDIO_RECORDING_SOURCE_USER_BUTTON) ==
           DENZIC_AUDIO_RECORDING_RESULT_CAPTURE_UNAVAILABLE);
    assert(context.state == DENZIC_AUDIO_RECORDING_STATE_PENDING_START);
    assert(context.pending_source == DENZIC_AUDIO_RECORDING_SOURCE_USER_BUTTON);
    assert(denzic_audio_recording_dispatch_v1(
               &context, DENZIC_AUDIO_RECORDING_EVENT_TOGGLE_REQUEST,
               DENZIC_AUDIO_RECORDING_SOURCE_USER_BUTTON) ==
           DENZIC_AUDIO_RECORDING_RESULT_OK);
    assert(context.state == DENZIC_AUDIO_RECORDING_STATE_CANCELED);
}

static void test_pending_capture_becomes_ready(void)
{
    denzic_audio_recording_context_v1_t context;
    denzic_audio_recording_init_v1(&context, NULL);
    denzic_audio_recording_set_capture_ready_v1(&context, false);
    (void)denzic_audio_recording_dispatch_v1(
        &context, DENZIC_AUDIO_RECORDING_EVENT_START_REQUEST,
        DENZIC_AUDIO_RECORDING_SOURCE_USER_TOUCH);
    denzic_audio_recording_set_capture_ready_v1(&context, true);
    assert(denzic_audio_recording_dispatch_v1(
               &context, DENZIC_AUDIO_RECORDING_EVENT_CAPTURE_READY,
               context.pending_source) == DENZIC_AUDIO_RECORDING_RESULT_OK);
    assert(context.state == DENZIC_AUDIO_RECORDING_STATE_RECORDING);
    assert(context.active_source == DENZIC_AUDIO_RECORDING_SOURCE_USER_TOUCH);
}

static void test_unexpected_inactive_session_enters_recovery(void)
{
    denzic_audio_recording_context_v1_t context;
    denzic_audio_recording_init_v1(&context, NULL);
    (void)denzic_audio_recording_dispatch_v1(
        &context, DENZIC_AUDIO_RECORDING_EVENT_START_REQUEST,
        DENZIC_AUDIO_RECORDING_SOURCE_HOST_CONTROL);
    assert(denzic_audio_recording_dispatch_v1(
               &context, DENZIC_AUDIO_RECORDING_EVENT_SESSION_FINISHED,
               DENZIC_AUDIO_RECORDING_SOURCE_HOST_CONTROL) ==
           DENZIC_AUDIO_RECORDING_RESULT_INTERNAL_ERROR);
    assert(context.state == DENZIC_AUDIO_RECORDING_STATE_ERROR_RECOVERY);
}

static void test_custom_format_and_artifact(void)
{
    denzic_audio_format_v1_t format = denzic_audio_recording_default_format_v1();
    format.source_family = DENZIC_AUDIO_SOURCE_SPH0655_PDM_COMPATIBLE;
    format.ingress = DENZIC_AUDIO_INGRESS_SAI_PDM;
    denzic_audio_recording_context_v1_t context;
    denzic_audio_recording_init_v1(&context, &format);
    assert(memcmp(&context.metadata.sample_format, &format, sizeof(format)) == 0);
    assert(denzic_audio_recording_transition_count_v1() >= 8u);
    assert(denzic_audio_recording_transition_at_v1(0u) != NULL);
    assert(denzic_audio_recording_transition_at_v1(
               denzic_audio_recording_transition_count_v1()) == NULL);
    assert(strcmp(denzic_audio_recording_state_name_v1(
                      DENZIC_AUDIO_RECORDING_STATE_RECORDING),
                  "recording") == 0);
}

int main(void)
{
    test_listener_style_toggle_session();
    test_pending_capture_and_cancel();
    test_pending_capture_becomes_ready();
    test_unexpected_inactive_session_enters_recovery();
    test_custom_format_and_artifact();
    return 0;
}
