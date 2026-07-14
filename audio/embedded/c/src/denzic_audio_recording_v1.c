#include "denzic_audio_recording_v1.h"

#include <string.h>

#define DENZIC_AUDIO_RECORDING_INITIAL_ID 1u

static const denzic_audio_recording_transition_case_v1_t
    DENZIC_AUDIO_RECORDING_TRANSITIONS[] = {
        {"start_idle", DENZIC_AUDIO_RECORDING_STATE_IDLE,
         DENZIC_AUDIO_RECORDING_EVENT_START_REQUEST, true, false, false, false,
         DENZIC_AUDIO_RECORDING_EFFECT_START_CAPTURE,
         DENZIC_AUDIO_RECORDING_STATE_RECORDING,
         DENZIC_AUDIO_RECORDING_RESULT_OK, "start_recording"},
        {"pending_capture", DENZIC_AUDIO_RECORDING_STATE_IDLE,
         DENZIC_AUDIO_RECORDING_EVENT_START_REQUEST, false, false, false, false,
         DENZIC_AUDIO_RECORDING_EFFECT_ENTER_PENDING_START,
         DENZIC_AUDIO_RECORDING_STATE_PENDING_START,
         DENZIC_AUDIO_RECORDING_RESULT_CAPTURE_UNAVAILABLE,
         "pending_start_capture_not_ready"},
        {"capture_ready", DENZIC_AUDIO_RECORDING_STATE_PENDING_START,
         DENZIC_AUDIO_RECORDING_EVENT_CAPTURE_READY, true, false, false, false,
         DENZIC_AUDIO_RECORDING_EFFECT_START_CAPTURE,
         DENZIC_AUDIO_RECORDING_STATE_RECORDING,
         DENZIC_AUDIO_RECORDING_RESULT_OK, "pending_start_capture_ready"},
        {"toggle_start", DENZIC_AUDIO_RECORDING_STATE_IDLE,
         DENZIC_AUDIO_RECORDING_EVENT_TOGGLE_REQUEST, true, false, false, false,
         DENZIC_AUDIO_RECORDING_EFFECT_START_CAPTURE,
         DENZIC_AUDIO_RECORDING_STATE_RECORDING,
         DENZIC_AUDIO_RECORDING_RESULT_OK, "toggle_start"},
        {"toggle_stop", DENZIC_AUDIO_RECORDING_STATE_RECORDING,
         DENZIC_AUDIO_RECORDING_EVENT_TOGGLE_REQUEST, true, true, false, false,
         DENZIC_AUDIO_RECORDING_EFFECT_STOP_CAPTURE,
         DENZIC_AUDIO_RECORDING_STATE_STOPPING,
         DENZIC_AUDIO_RECORDING_RESULT_OK, "toggle_stop"},
        {"session_finished", DENZIC_AUDIO_RECORDING_STATE_STOPPING,
         DENZIC_AUDIO_RECORDING_EVENT_SESSION_FINISHED, true, false, true, false,
         DENZIC_AUDIO_RECORDING_EFFECT_MARK_TRANSFER_READY,
         DENZIC_AUDIO_RECORDING_STATE_TRANSFER_READY,
         DENZIC_AUDIO_RECORDING_RESULT_OK, "recording_session_finished"},
        {"cancel_pending", DENZIC_AUDIO_RECORDING_STATE_PENDING_START,
         DENZIC_AUDIO_RECORDING_EVENT_CANCEL_REQUEST, false, false, false, false,
         DENZIC_AUDIO_RECORDING_EFFECT_CANCEL_CAPTURE,
         DENZIC_AUDIO_RECORDING_STATE_CANCELED,
         DENZIC_AUDIO_RECORDING_RESULT_OK, "pending_start_canceled"},
        {"abort_without_stop", DENZIC_AUDIO_RECORDING_STATE_RECORDING,
         DENZIC_AUDIO_RECORDING_EVENT_SESSION_FINISHED, true, false, false, false,
         DENZIC_AUDIO_RECORDING_EFFECT_ENTER_ERROR_RECOVERY,
         DENZIC_AUDIO_RECORDING_STATE_ERROR_RECOVERY,
         DENZIC_AUDIO_RECORDING_RESULT_INTERNAL_ERROR,
         "session_finished_without_stop"},
};

static denzic_audio_recording_decision_v1_t make_decision(
    denzic_audio_recording_effect_v1_t effect,
    denzic_audio_recording_state_v1_t next_state,
    denzic_audio_recording_result_v1_t result,
    const char *detail)
{
    denzic_audio_recording_decision_v1_t decision = {
        .effect = effect,
        .next_state = next_state,
        .result = result,
        .detail = detail,
    };
    return decision;
}

denzic_audio_format_v1_t denzic_audio_recording_default_format_v1(void)
{
    denzic_audio_format_v1_t format = {
        .source_family = DENZIC_AUDIO_SOURCE_UNKNOWN,
        .ingress = DENZIC_AUDIO_INGRESS_UNKNOWN,
        .encoding = DENZIC_AUDIO_ENCODING_PCM16_MONO,
        .sample_rate_hz = DENZIC_AUDIO_RECORDING_V1_SAMPLE_RATE_HZ,
        .bits_per_sample = DENZIC_AUDIO_RECORDING_V1_BITS_PER_SAMPLE,
        .channels = DENZIC_AUDIO_RECORDING_V1_CHANNELS,
        .frame_samples = DENZIC_AUDIO_RECORDING_V1_FRAME_SAMPLES,
    };
    return format;
}

static void reset_metadata(denzic_audio_recording_context_v1_t *context)
{
    memset(&context->metadata, 0, sizeof(context->metadata));
    context->metadata.sample_format = context->default_format;
    context->metadata.sync_state = DENZIC_AUDIO_RECORDING_SYNC_UNSYNCED;
}

void denzic_audio_recording_init_v1(
    denzic_audio_recording_context_v1_t *context,
    const denzic_audio_format_v1_t *format)
{
    if (context == NULL) {
        return;
    }
    memset(context, 0, sizeof(*context));
    context->state = DENZIC_AUDIO_RECORDING_STATE_IDLE;
    context->next_recording_id = DENZIC_AUDIO_RECORDING_INITIAL_ID;
    context->capture_ready = true;
    context->default_format = format != NULL
                                  ? *format
                                  : denzic_audio_recording_default_format_v1();
    reset_metadata(context);
}

void denzic_audio_recording_set_capture_ready_v1(
    denzic_audio_recording_context_v1_t *context,
    bool ready)
{
    if (context != NULL) {
        context->capture_ready = ready;
    }
}

static denzic_audio_recording_snapshot_v1_t snapshot_from_context(
    const denzic_audio_recording_context_v1_t *context)
{
    denzic_audio_recording_snapshot_v1_t snapshot = {
        .state = context != NULL ? context->state
                                 : DENZIC_AUDIO_RECORDING_STATE_ERROR_RECOVERY,
        .capture_ready = context != NULL && context->capture_ready,
        .active_capture = false,
        .stop_requested = false,
        .transfer_window_open = false,
    };
    if (context == NULL) {
        return snapshot;
    }
    snapshot.active_capture =
        context->state == DENZIC_AUDIO_RECORDING_STATE_RECORDING ||
        context->state == DENZIC_AUDIO_RECORDING_STATE_STOPPING;
    snapshot.stop_requested =
        context->state == DENZIC_AUDIO_RECORDING_STATE_STOPPING;
    snapshot.transfer_window_open =
        context->state == DENZIC_AUDIO_RECORDING_STATE_TRANSFER_READY ||
        context->state == DENZIC_AUDIO_RECORDING_STATE_TRANSFER_ACTIVE;
    return snapshot;
}

denzic_audio_recording_decision_v1_t denzic_audio_recording_decide_v1(
    denzic_audio_recording_event_v1_t event,
    const denzic_audio_recording_snapshot_v1_t *snapshot)
{
    const denzic_audio_recording_state_v1_t state =
        snapshot != NULL ? snapshot->state
                         : DENZIC_AUDIO_RECORDING_STATE_ERROR_RECOVERY;

    if (event == DENZIC_AUDIO_RECORDING_EVENT_TOGGLE_REQUEST) {
        if (state == DENZIC_AUDIO_RECORDING_STATE_RECORDING) {
            event = DENZIC_AUDIO_RECORDING_EVENT_STOP_REQUEST;
        } else if (state == DENZIC_AUDIO_RECORDING_STATE_PENDING_START) {
            event = DENZIC_AUDIO_RECORDING_EVENT_CANCEL_REQUEST;
        } else {
            event = DENZIC_AUDIO_RECORDING_EVENT_START_REQUEST;
        }
    }

    if (event == DENZIC_AUDIO_RECORDING_EVENT_ERROR_DETECTED) {
        return make_decision(
            DENZIC_AUDIO_RECORDING_EFFECT_ENTER_ERROR_RECOVERY,
            DENZIC_AUDIO_RECORDING_STATE_ERROR_RECOVERY,
            DENZIC_AUDIO_RECORDING_RESULT_INTERNAL_ERROR,
            "recording_error_recovery");
    }
    if (event == DENZIC_AUDIO_RECORDING_EVENT_RECOVERY_COMPLETE) {
        if (state == DENZIC_AUDIO_RECORDING_STATE_ERROR_RECOVERY ||
            state == DENZIC_AUDIO_RECORDING_STATE_CANCELED ||
            state == DENZIC_AUDIO_RECORDING_STATE_TRANSFERRED) {
            return make_decision(
                DENZIC_AUDIO_RECORDING_EFFECT_RECOVER_TO_IDLE,
                DENZIC_AUDIO_RECORDING_STATE_IDLE,
                DENZIC_AUDIO_RECORDING_RESULT_OK,
                "recording_recovered");
        }
        return make_decision(DENZIC_AUDIO_RECORDING_EFFECT_IGNORE, state,
                             DENZIC_AUDIO_RECORDING_RESULT_INVALID_STATE,
                             "recovery_ignored_not_needed");
    }
    if (event == DENZIC_AUDIO_RECORDING_EVENT_START_REQUEST) {
        if (state == DENZIC_AUDIO_RECORDING_STATE_IDLE ||
            state == DENZIC_AUDIO_RECORDING_STATE_CANCELED ||
            state == DENZIC_AUDIO_RECORDING_STATE_TRANSFERRED) {
            if (snapshot != NULL && snapshot->capture_ready) {
                return make_decision(
                    DENZIC_AUDIO_RECORDING_EFFECT_START_CAPTURE,
                    DENZIC_AUDIO_RECORDING_STATE_RECORDING,
                    DENZIC_AUDIO_RECORDING_RESULT_OK, "start_recording");
            }
            return make_decision(
                DENZIC_AUDIO_RECORDING_EFFECT_ENTER_PENDING_START,
                DENZIC_AUDIO_RECORDING_STATE_PENDING_START,
                DENZIC_AUDIO_RECORDING_RESULT_CAPTURE_UNAVAILABLE,
                "pending_start_capture_not_ready");
        }
        return make_decision(
            DENZIC_AUDIO_RECORDING_EFFECT_REJECT, state,
            DENZIC_AUDIO_RECORDING_RESULT_BUSY,
            state == DENZIC_AUDIO_RECORDING_STATE_STOPPING
                ? "start_rejected_stop_in_progress"
                : "start_rejected_busy");
    }
    if (event == DENZIC_AUDIO_RECORDING_EVENT_CAPTURE_READY) {
        if (state == DENZIC_AUDIO_RECORDING_STATE_PENDING_START) {
            return make_decision(
                DENZIC_AUDIO_RECORDING_EFFECT_START_CAPTURE,
                DENZIC_AUDIO_RECORDING_STATE_RECORDING,
                DENZIC_AUDIO_RECORDING_RESULT_OK,
                "pending_start_capture_ready");
        }
        return make_decision(DENZIC_AUDIO_RECORDING_EFFECT_IGNORE, state,
                             DENZIC_AUDIO_RECORDING_RESULT_INVALID_STATE,
                             "capture_ready_ignored");
    }
    if (event == DENZIC_AUDIO_RECORDING_EVENT_STOP_REQUEST) {
        if (state == DENZIC_AUDIO_RECORDING_STATE_RECORDING) {
            return make_decision(DENZIC_AUDIO_RECORDING_EFFECT_STOP_CAPTURE,
                                 DENZIC_AUDIO_RECORDING_STATE_STOPPING,
                                 DENZIC_AUDIO_RECORDING_RESULT_OK,
                                 "stop_requested");
        }
        if (state == DENZIC_AUDIO_RECORDING_STATE_PENDING_START) {
            return make_decision(DENZIC_AUDIO_RECORDING_EFFECT_CANCEL_CAPTURE,
                                 DENZIC_AUDIO_RECORDING_STATE_CANCELED,
                                 DENZIC_AUDIO_RECORDING_RESULT_OK,
                                 "stop_cleared_pending_start");
        }
        return make_decision(DENZIC_AUDIO_RECORDING_EFFECT_REJECT, state,
                             DENZIC_AUDIO_RECORDING_RESULT_INVALID_STATE,
                             "stop_rejected_no_active_session");
    }
    if (event == DENZIC_AUDIO_RECORDING_EVENT_CANCEL_REQUEST) {
        if (state == DENZIC_AUDIO_RECORDING_STATE_PENDING_START ||
            state == DENZIC_AUDIO_RECORDING_STATE_RECORDING ||
            state == DENZIC_AUDIO_RECORDING_STATE_STOPPING ||
            state == DENZIC_AUDIO_RECORDING_STATE_TRANSFER_READY ||
            state == DENZIC_AUDIO_RECORDING_STATE_TRANSFER_ACTIVE) {
            return make_decision(
                DENZIC_AUDIO_RECORDING_EFFECT_CANCEL_CAPTURE,
                DENZIC_AUDIO_RECORDING_STATE_CANCELED,
                DENZIC_AUDIO_RECORDING_RESULT_OK,
                state == DENZIC_AUDIO_RECORDING_STATE_PENDING_START
                    ? "pending_start_canceled"
                    : "recording_canceled");
        }
        return make_decision(DENZIC_AUDIO_RECORDING_EFFECT_IGNORE, state,
                             DENZIC_AUDIO_RECORDING_RESULT_INVALID_STATE,
                             "cancel_ignored_no_active_session");
    }
    if (event == DENZIC_AUDIO_RECORDING_EVENT_SESSION_FINISHED) {
        if (state == DENZIC_AUDIO_RECORDING_STATE_STOPPING) {
            return make_decision(
                DENZIC_AUDIO_RECORDING_EFFECT_MARK_TRANSFER_READY,
                DENZIC_AUDIO_RECORDING_STATE_TRANSFER_READY,
                DENZIC_AUDIO_RECORDING_RESULT_OK,
                "recording_session_finished");
        }
        if (state == DENZIC_AUDIO_RECORDING_STATE_RECORDING) {
            return make_decision(
                DENZIC_AUDIO_RECORDING_EFFECT_ENTER_ERROR_RECOVERY,
                DENZIC_AUDIO_RECORDING_STATE_ERROR_RECOVERY,
                DENZIC_AUDIO_RECORDING_RESULT_INTERNAL_ERROR,
                "session_finished_without_stop");
        }
        return make_decision(DENZIC_AUDIO_RECORDING_EFFECT_IGNORE, state,
                             DENZIC_AUDIO_RECORDING_RESULT_INVALID_STATE,
                             "session_finished_ignored");
    }
    if (event == DENZIC_AUDIO_RECORDING_EVENT_TRANSFER_BEGIN) {
        if (state == DENZIC_AUDIO_RECORDING_STATE_TRANSFER_READY) {
            return make_decision(
                DENZIC_AUDIO_RECORDING_EFFECT_BEGIN_TRANSFER,
                DENZIC_AUDIO_RECORDING_STATE_TRANSFER_ACTIVE,
                DENZIC_AUDIO_RECORDING_RESULT_OK,
                "recording_transfer_active");
        }
        return make_decision(DENZIC_AUDIO_RECORDING_EFFECT_REJECT, state,
                             DENZIC_AUDIO_RECORDING_RESULT_INVALID_STATE,
                             "transfer_begin_rejected");
    }
    if (event == DENZIC_AUDIO_RECORDING_EVENT_TRANSFER_FINISHED) {
        if (state == DENZIC_AUDIO_RECORDING_STATE_TRANSFER_ACTIVE) {
            return make_decision(
                DENZIC_AUDIO_RECORDING_EFFECT_FINISH_TRANSFER,
                DENZIC_AUDIO_RECORDING_STATE_TRANSFERRED,
                DENZIC_AUDIO_RECORDING_RESULT_OK, "recording_transferred");
        }
        return make_decision(DENZIC_AUDIO_RECORDING_EFFECT_REJECT, state,
                             DENZIC_AUDIO_RECORDING_RESULT_INVALID_STATE,
                             "transfer_finished_rejected");
    }
    return make_decision(DENZIC_AUDIO_RECORDING_EFFECT_REJECT, state,
                         DENZIC_AUDIO_RECORDING_RESULT_BAD_ARGUMENT,
                         "unknown_recording_event");
}

static void begin_metadata(denzic_audio_recording_context_v1_t *context)
{
    reset_metadata(context);
    context->metadata.recording_id = context->next_recording_id++;
}

static void apply_decision(
    denzic_audio_recording_context_v1_t *context,
    denzic_audio_recording_source_v1_t source,
    const denzic_audio_recording_decision_v1_t *decision)
{
    context->state = decision->next_state;
    switch (decision->effect) {
    case DENZIC_AUDIO_RECORDING_EFFECT_ENTER_PENDING_START:
        context->pending_source = source;
        break;
    case DENZIC_AUDIO_RECORDING_EFFECT_START_CAPTURE:
        context->active_source = source;
        context->pending_source = DENZIC_AUDIO_RECORDING_SOURCE_UNKNOWN;
        begin_metadata(context);
        break;
    case DENZIC_AUDIO_RECORDING_EFFECT_STOP_CAPTURE:
        context->metadata.byte_offset_ready = context->metadata.total_length;
        break;
    case DENZIC_AUDIO_RECORDING_EFFECT_CANCEL_CAPTURE:
        context->metadata.sync_state = DENZIC_AUDIO_RECORDING_SYNC_DELETED;
        context->active_source = DENZIC_AUDIO_RECORDING_SOURCE_UNKNOWN;
        context->pending_source = DENZIC_AUDIO_RECORDING_SOURCE_UNKNOWN;
        break;
    case DENZIC_AUDIO_RECORDING_EFFECT_MARK_TRANSFER_READY:
        context->metadata.byte_offset_ready = context->metadata.total_length;
        context->metadata.sync_state = DENZIC_AUDIO_RECORDING_SYNC_UNSYNCED;
        break;
    case DENZIC_AUDIO_RECORDING_EFFECT_BEGIN_TRANSFER:
        context->metadata.sync_state =
            DENZIC_AUDIO_RECORDING_SYNC_TRANSFER_ACTIVE;
        context->metadata.packet_sequence_next = 0u;
        break;
    case DENZIC_AUDIO_RECORDING_EFFECT_FINISH_TRANSFER:
        context->metadata.sync_state = DENZIC_AUDIO_RECORDING_SYNC_TRANSFERRED;
        context->metadata.byte_offset_ready = context->metadata.total_length;
        context->active_source = DENZIC_AUDIO_RECORDING_SOURCE_UNKNOWN;
        break;
    case DENZIC_AUDIO_RECORDING_EFFECT_ENTER_ERROR_RECOVERY:
        context->metadata.sync_state = DENZIC_AUDIO_RECORDING_SYNC_UNSYNCED;
        break;
    case DENZIC_AUDIO_RECORDING_EFFECT_RECOVER_TO_IDLE:
        context->active_source = DENZIC_AUDIO_RECORDING_SOURCE_UNKNOWN;
        context->pending_source = DENZIC_AUDIO_RECORDING_SOURCE_UNKNOWN;
        break;
    default:
        break;
    }
}

denzic_audio_recording_result_v1_t denzic_audio_recording_dispatch_v1(
    denzic_audio_recording_context_v1_t *context,
    denzic_audio_recording_event_v1_t event,
    denzic_audio_recording_source_v1_t source)
{
    if (context == NULL) {
        return DENZIC_AUDIO_RECORDING_RESULT_BAD_ARGUMENT;
    }
    const denzic_audio_recording_snapshot_v1_t snapshot =
        snapshot_from_context(context);
    const denzic_audio_recording_decision_v1_t decision =
        denzic_audio_recording_decide_v1(event, &snapshot);
    apply_decision(context, source, &decision);
    return decision.result;
}

denzic_audio_recording_result_v1_t denzic_audio_recording_feed_v1(
    denzic_audio_recording_context_v1_t *context,
    const denzic_audio_frame_batch_v1_t *batch)
{
    if (context == NULL || batch == NULL) {
        return DENZIC_AUDIO_RECORDING_RESULT_BAD_ARGUMENT;
    }
    if (context->state != DENZIC_AUDIO_RECORDING_STATE_RECORDING) {
        return DENZIC_AUDIO_RECORDING_RESULT_INVALID_STATE;
    }
    const uint32_t bytes =
        batch->frame_count * (uint32_t)batch->bytes_per_frame;
    context->metadata.frame_count += batch->frame_count;
    context->metadata.duration_ms += batch->duration_ms;
    context->metadata.total_length += bytes;
    context->metadata.content_crc32 ^= batch->crc32;
    context->metadata.dropped_frames += batch->dropped_frames;
    context->metadata.sha256_prefix_words[0] ^= batch->crc32;
    context->metadata.sha256_prefix_words[1] += bytes;
    return DENZIC_AUDIO_RECORDING_RESULT_OK;
}

const denzic_audio_recording_metadata_v1_t *denzic_audio_recording_metadata_v1(
    const denzic_audio_recording_context_v1_t *context)
{
    return context != NULL ? &context->metadata : NULL;
}

size_t denzic_audio_recording_transition_count_v1(void)
{
    return sizeof(DENZIC_AUDIO_RECORDING_TRANSITIONS) /
           sizeof(DENZIC_AUDIO_RECORDING_TRANSITIONS[0]);
}

const denzic_audio_recording_transition_case_v1_t *
denzic_audio_recording_transition_at_v1(size_t index)
{
    if (index >= denzic_audio_recording_transition_count_v1()) {
        return NULL;
    }
    return &DENZIC_AUDIO_RECORDING_TRANSITIONS[index];
}

const char *denzic_audio_recording_state_name_v1(
    denzic_audio_recording_state_v1_t state)
{
    static const char *const names[] = {
        "idle", "pending_start", "recording", "stopping", "canceled",
        "transfer_ready", "transfer_active", "transferred", "error_recovery",
    };
    return (unsigned)state < (sizeof(names) / sizeof(names[0]))
               ? names[(unsigned)state]
               : "unknown";
}

const char *denzic_audio_recording_event_name_v1(
    denzic_audio_recording_event_v1_t event)
{
    static const char *const names[] = {
        "start_request", "capture_ready", "stop_request", "cancel_request",
        "session_finished", "transfer_begin", "transfer_finished",
        "error_detected", "recovery_complete", "toggle_request",
    };
    return (unsigned)event < (sizeof(names) / sizeof(names[0]))
               ? names[(unsigned)event]
               : "unknown";
}

const char *denzic_audio_recording_effect_name_v1(
    denzic_audio_recording_effect_v1_t effect)
{
    static const char *const names[] = {
        "ignore", "enter_pending_start", "start_capture", "stop_capture",
        "cancel_capture", "mark_transfer_ready", "begin_transfer",
        "finish_transfer", "enter_error_recovery", "recover_to_idle", "reject",
    };
    return (unsigned)effect < (sizeof(names) / sizeof(names[0]))
               ? names[(unsigned)effect]
               : "unknown";
}
