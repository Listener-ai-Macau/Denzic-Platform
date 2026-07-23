#include "denzic_device_control_v1.h"

#include <inttypes.h>
#include <stdio.h>
#include <string.h>

static denzic_device_control_v1_decision_t decision(
    denzic_device_control_v1_operation_result_t result,
    denzic_device_control_v1_error_category_t error,
    bool execute,
    bool replayed)
{
    denzic_device_control_v1_decision_t value;
    value.result = result;
    value.error = error;
    value.execute = execute;
    value.replayed = replayed;
    return value;
}

static denzic_device_control_v1_decision_t reject(
    denzic_device_control_v1_error_category_t error)
{
    return decision(DENZIC_DEVICE_CONTROL_V1_OPERATION_RESULT_REJECTED, error, false, false);
}

static bool matches(
    uint64_t operation_id,
    uint64_t idempotency_key,
    uint64_t candidate_operation_id,
    uint64_t candidate_idempotency_key)
{
    return operation_id == candidate_operation_id && idempotency_key == candidate_idempotency_key;
}

static denzic_device_control_v1_terminal_record_t *find_terminal(
    denzic_device_control_v1_context_t *context,
    uint64_t operation_id,
    uint64_t idempotency_key)
{
    size_t index;
    if (context == NULL) {
        return NULL;
    }
    for (index = 0u; index < DENZIC_DEVICE_CONTROL_V1_TERMINAL_HISTORY_CAPACITY; ++index) {
        denzic_device_control_v1_terminal_record_t *record = &context->terminal_history[index];
        if (record->valid && matches(operation_id, idempotency_key, record->operation_id, record->idempotency_key)) {
            return record;
        }
    }
    return NULL;
}

static denzic_device_control_v1_decision_t replay_terminal(
    const denzic_device_control_v1_terminal_record_t *record)
{
    return decision(record->result, record->error, false, true);
}

static bool active_matches(
    const denzic_device_control_v1_context_t *context,
    uint64_t operation_id,
    uint64_t idempotency_key)
{
    return context != NULL &&
           context->has_active_operation &&
           matches(
               operation_id,
               idempotency_key,
               context->active_request.operation_id,
               context->active_request.idempotency_key);
}

static void finish_active(
    denzic_device_control_v1_context_t *context,
    denzic_device_control_v1_operation_result_t result,
    denzic_device_control_v1_error_category_t error)
{
    denzic_device_control_v1_terminal_record_t *record;
    if (context == NULL || !context->has_active_operation) {
        return;
    }
    record = &context->terminal_history[context->terminal_history_next];
    record->operation_id = context->active_request.operation_id;
    record->idempotency_key = context->active_request.idempotency_key;
    record->kind = context->active_request.kind;
    record->result = result;
    record->error = error;
    record->valid = true;
    context->terminal_history_next =
        (context->terminal_history_next + 1u) % DENZIC_DEVICE_CONTROL_V1_TERMINAL_HISTORY_CAPACITY;
    context->has_active_operation = false;
    memset(&context->active_request, 0, sizeof(context->active_request));
    context->active_deadline_ms = 0u;
    context->active_write_acknowledged = false;
    context->active_write_acknowledged_revision = 0u;
}

static bool lifecycle_accepts_operation(
    denzic_device_control_v1_lifecycle_state_t lifecycle,
    denzic_device_control_v1_operation_kind_t kind)
{
    switch (kind) {
        case DENZIC_DEVICE_CONTROL_V1_OPERATION_KIND_DISCOVER:
            return lifecycle == DENZIC_DEVICE_CONTROL_V1_LIFECYCLE_STATE_DISCONNECTED ||
                   lifecycle == DENZIC_DEVICE_CONTROL_V1_LIFECYCLE_STATE_RECOVERING;
        case DENZIC_DEVICE_CONTROL_V1_OPERATION_KIND_CONNECT:
            return lifecycle == DENZIC_DEVICE_CONTROL_V1_LIFECYCLE_STATE_DISCONNECTED ||
                   lifecycle == DENZIC_DEVICE_CONTROL_V1_LIFECYCLE_STATE_DISCOVERING ||
                   lifecycle == DENZIC_DEVICE_CONTROL_V1_LIFECYCLE_STATE_RECOVERING;
        case DENZIC_DEVICE_CONTROL_V1_OPERATION_KIND_SECURE:
            return lifecycle == DENZIC_DEVICE_CONTROL_V1_LIFECYCLE_STATE_CONNECTING ||
                   lifecycle == DENZIC_DEVICE_CONTROL_V1_LIFECYCLE_STATE_SECURING;
        case DENZIC_DEVICE_CONTROL_V1_OPERATION_KIND_READ_CAPABILITIES:
        case DENZIC_DEVICE_CONTROL_V1_OPERATION_KIND_READ_SETTING:
        case DENZIC_DEVICE_CONTROL_V1_OPERATION_KIND_WRITE_SETTING:
        case DENZIC_DEVICE_CONTROL_V1_OPERATION_KIND_INVOKE_COMMAND:
            return lifecycle == DENZIC_DEVICE_CONTROL_V1_LIFECYCLE_STATE_READY;
        default:
            return false;
    }
}

static bool terminal_result(denzic_device_control_v1_operation_result_t result)
{
    return result == DENZIC_DEVICE_CONTROL_V1_OPERATION_RESULT_SUCCEEDED ||
           result == DENZIC_DEVICE_CONTROL_V1_OPERATION_RESULT_CANCELLED ||
           result == DENZIC_DEVICE_CONTROL_V1_OPERATION_RESULT_TIMED_OUT ||
           result == DENZIC_DEVICE_CONTROL_V1_OPERATION_RESULT_TRANSPORT_LOST ||
           result == DENZIC_DEVICE_CONTROL_V1_OPERATION_RESULT_FAILED;
}

void denzic_device_control_v1_init(
    denzic_device_control_v1_context_t *context,
    denzic_device_control_v1_transport_t transport)
{
    if (context == NULL) {
        return;
    }
    memset(context, 0, sizeof(*context));
    context->transport = transport;
    context->lifecycle = DENZIC_DEVICE_CONTROL_V1_LIFECYCLE_STATE_DISCONNECTED;
    context->ownership = DENZIC_DEVICE_CONTROL_V1_OWNERSHIP_STATE_UNKNOWN;
}

void denzic_device_control_v1_set_lifecycle(
    denzic_device_control_v1_context_t *context,
    denzic_device_control_v1_lifecycle_state_t lifecycle)
{
    if (context == NULL) {
        return;
    }
    if (lifecycle == DENZIC_DEVICE_CONTROL_V1_LIFECYCLE_STATE_OWNED_ELSEWHERE) {
        denzic_device_control_v1_set_ownership(
            context,
            DENZIC_DEVICE_CONTROL_V1_OWNERSHIP_STATE_EXTERNAL);
        return;
    }
    if (context->ownership == DENZIC_DEVICE_CONTROL_V1_OWNERSHIP_STATE_EXTERNAL) {
        return;
    }
    context->lifecycle = lifecycle;
}

void denzic_device_control_v1_set_ownership(
    denzic_device_control_v1_context_t *context,
    denzic_device_control_v1_ownership_state_t ownership)
{
    if (context == NULL) {
        return;
    }
    context->ownership = ownership;
    if (ownership == DENZIC_DEVICE_CONTROL_V1_OWNERSHIP_STATE_EXTERNAL) {
        context->lifecycle = DENZIC_DEVICE_CONTROL_V1_LIFECYCLE_STATE_OWNED_ELSEWHERE;
        finish_active(
            context,
            DENZIC_DEVICE_CONTROL_V1_OPERATION_RESULT_FAILED,
            DENZIC_DEVICE_CONTROL_V1_ERROR_CATEGORY_OWNERSHIP);
    } else if (ownership == DENZIC_DEVICE_CONTROL_V1_OWNERSHIP_STATE_MANUAL_UNPAIRED) {
        context->lifecycle = DENZIC_DEVICE_CONTROL_V1_LIFECYCLE_STATE_DISCONNECTED;
        finish_active(
            context,
            DENZIC_DEVICE_CONTROL_V1_OPERATION_RESULT_CANCELLED,
            DENZIC_DEVICE_CONTROL_V1_ERROR_CATEGORY_OWNERSHIP);
    }
}

denzic_device_control_v1_decision_t denzic_device_control_v1_begin(
    denzic_device_control_v1_context_t *context,
    const denzic_device_control_v1_request_t *request,
    uint32_t now_ms)
{
    denzic_device_control_v1_terminal_record_t *record;
    if (context == NULL || request == NULL || request->operation_id == 0u ||
        request->idempotency_key == 0u || request->timeout_ms == 0u ||
        request->kind == DENZIC_DEVICE_CONTROL_V1_OPERATION_KIND_UNKNOWN) {
        return reject(DENZIC_DEVICE_CONTROL_V1_ERROR_CATEGORY_PROTOCOL);
    }
    if (active_matches(context, request->operation_id, request->idempotency_key)) {
        return decision(
            DENZIC_DEVICE_CONTROL_V1_OPERATION_RESULT_ACCEPTED,
            DENZIC_DEVICE_CONTROL_V1_ERROR_CATEGORY_NONE,
            false,
            true);
    }
    if (context->has_active_operation) {
        return reject(DENZIC_DEVICE_CONTROL_V1_ERROR_CATEGORY_RESOURCE);
    }
    record = find_terminal(context, request->operation_id, request->idempotency_key);
    if (record != NULL) {
        return replay_terminal(record);
    }
    if (context->ownership == DENZIC_DEVICE_CONTROL_V1_OWNERSHIP_STATE_EXTERNAL ||
        context->lifecycle == DENZIC_DEVICE_CONTROL_V1_LIFECYCLE_STATE_OWNED_ELSEWHERE) {
        return reject(DENZIC_DEVICE_CONTROL_V1_ERROR_CATEGORY_OWNERSHIP);
    }
    if (request->automatic &&
        (context->ownership == DENZIC_DEVICE_CONTROL_V1_OWNERSHIP_STATE_MANUAL_UNPAIRED ||
         (!request->recovery_authorized &&
          context->ownership != DENZIC_DEVICE_CONTROL_V1_OWNERSHIP_STATE_LOCAL))) {
        return reject(DENZIC_DEVICE_CONTROL_V1_ERROR_CATEGORY_OWNERSHIP);
    }
    if (!lifecycle_accepts_operation(context->lifecycle, request->kind)) {
        return reject(
            context->lifecycle == DENZIC_DEVICE_CONTROL_V1_LIFECYCLE_STATE_FAILED
                ? DENZIC_DEVICE_CONTROL_V1_ERROR_CATEGORY_DEVICE
                : DENZIC_DEVICE_CONTROL_V1_ERROR_CATEGORY_PROTOCOL);
    }
    context->active_request = *request;
    context->has_active_operation = true;
    context->active_deadline_ms = now_ms + request->timeout_ms;
    context->active_write_acknowledged = false;
    context->active_write_acknowledged_revision = 0u;
    return decision(
        DENZIC_DEVICE_CONTROL_V1_OPERATION_RESULT_ACCEPTED,
        DENZIC_DEVICE_CONTROL_V1_ERROR_CATEGORY_NONE,
        true,
        false);
}

denzic_device_control_v1_decision_t denzic_device_control_v1_cancel(
    denzic_device_control_v1_context_t *context,
    uint64_t operation_id,
    uint64_t idempotency_key)
{
    denzic_device_control_v1_terminal_record_t *record;
    if (active_matches(context, operation_id, idempotency_key)) {
        finish_active(
            context,
            DENZIC_DEVICE_CONTROL_V1_OPERATION_RESULT_CANCELLED,
            DENZIC_DEVICE_CONTROL_V1_ERROR_CATEGORY_NONE);
        return decision(
            DENZIC_DEVICE_CONTROL_V1_OPERATION_RESULT_CANCELLED,
            DENZIC_DEVICE_CONTROL_V1_ERROR_CATEGORY_NONE,
            false,
            false);
    }
    record = find_terminal(context, operation_id, idempotency_key);
    return record == NULL ? reject(DENZIC_DEVICE_CONTROL_V1_ERROR_CATEGORY_PROTOCOL)
                          : replay_terminal(record);
}

denzic_device_control_v1_decision_t denzic_device_control_v1_complete(
    denzic_device_control_v1_context_t *context,
    uint64_t operation_id,
    uint64_t idempotency_key,
    denzic_device_control_v1_operation_result_t result,
    denzic_device_control_v1_error_category_t error)
{
    denzic_device_control_v1_terminal_record_t *record;
    if (!active_matches(context, operation_id, idempotency_key)) {
        record = find_terminal(context, operation_id, idempotency_key);
        return record == NULL ? reject(DENZIC_DEVICE_CONTROL_V1_ERROR_CATEGORY_PROTOCOL)
                              : replay_terminal(record);
    }
    if (!terminal_result(result)) {
        return reject(DENZIC_DEVICE_CONTROL_V1_ERROR_CATEGORY_PROTOCOL);
    }
    if (context->active_request.kind == DENZIC_DEVICE_CONTROL_V1_OPERATION_KIND_WRITE_SETTING &&
        result == DENZIC_DEVICE_CONTROL_V1_OPERATION_RESULT_SUCCEEDED) {
        return reject(DENZIC_DEVICE_CONTROL_V1_ERROR_CATEGORY_PROTOCOL);
    }
    finish_active(context, result, error);
    return decision(result, error, false, false);
}

bool denzic_device_control_v1_expire(
    denzic_device_control_v1_context_t *context,
    uint32_t now_ms)
{
    if (context == NULL || !context->has_active_operation ||
        (int32_t)(now_ms - context->active_deadline_ms) < 0) {
        return false;
    }
    finish_active(
        context,
        DENZIC_DEVICE_CONTROL_V1_OPERATION_RESULT_TIMED_OUT,
        DENZIC_DEVICE_CONTROL_V1_ERROR_CATEGORY_TIMEOUT);
    return true;
}

bool denzic_device_control_v1_report_transport_lost(
    denzic_device_control_v1_context_t *context)
{
    bool had_active;
    if (context == NULL) {
        return false;
    }
    had_active = context->has_active_operation;
    context->lifecycle = DENZIC_DEVICE_CONTROL_V1_LIFECYCLE_STATE_DISCONNECTED;
    finish_active(
        context,
        DENZIC_DEVICE_CONTROL_V1_OPERATION_RESULT_TRANSPORT_LOST,
        DENZIC_DEVICE_CONTROL_V1_ERROR_CATEGORY_TRANSPORT);
    return had_active;
}

bool denzic_device_control_v1_report_capabilities(
    denzic_device_control_v1_context_t *context,
    uint64_t operation_id,
    uint64_t idempotency_key,
    uint32_t revision,
    uint64_t capability_mask)
{
    if (!active_matches(context, operation_id, idempotency_key) ||
        context->active_request.kind != DENZIC_DEVICE_CONTROL_V1_OPERATION_KIND_READ_CAPABILITIES) {
        return false;
    }
    context->capability_revision = revision;
    context->capability_mask = capability_mask;
    finish_active(
        context,
        DENZIC_DEVICE_CONTROL_V1_OPERATION_RESULT_SUCCEEDED,
        DENZIC_DEVICE_CONTROL_V1_ERROR_CATEGORY_NONE);
    return true;
}

bool denzic_device_control_v1_acknowledge_setting_write(
    denzic_device_control_v1_context_t *context,
    uint64_t operation_id,
    uint64_t idempotency_key,
    uint32_t acknowledged_revision)
{
    if (!active_matches(context, operation_id, idempotency_key) ||
        context->active_request.kind != DENZIC_DEVICE_CONTROL_V1_OPERATION_KIND_WRITE_SETTING ||
        acknowledged_revision < context->active_request.expected_settings_revision) {
        return false;
    }
    context->active_write_acknowledged = true;
    context->active_write_acknowledged_revision = acknowledged_revision;
    return true;
}

bool denzic_device_control_v1_confirm_setting_readback(
    denzic_device_control_v1_context_t *context,
    uint64_t operation_id,
    uint64_t idempotency_key,
    uint32_t observed_revision)
{
    if (!active_matches(context, operation_id, idempotency_key) ||
        (context->active_request.kind != DENZIC_DEVICE_CONTROL_V1_OPERATION_KIND_READ_SETTING &&
         context->active_request.kind != DENZIC_DEVICE_CONTROL_V1_OPERATION_KIND_WRITE_SETTING) ||
        observed_revision < context->settings_revision) {
        return false;
    }
    if (context->active_request.kind == DENZIC_DEVICE_CONTROL_V1_OPERATION_KIND_WRITE_SETTING &&
        (!context->active_write_acknowledged ||
         observed_revision < context->active_write_acknowledged_revision)) {
        return false;
    }
    context->settings_revision = observed_revision;
    finish_active(
        context,
        DENZIC_DEVICE_CONTROL_V1_OPERATION_RESULT_SUCCEEDED,
        DENZIC_DEVICE_CONTROL_V1_ERROR_CATEGORY_NONE);
    return true;
}

bool denzic_device_control_v1_format_settings_revision(
    char *out,
    size_t out_size,
    uint32_t revision)
{
    int written;

    if (out == NULL || out_size == 0u) {
        return false;
    }
    out[0] = '\0';
    if (revision == 0u) {
        return false;
    }
    written = snprintf(
        out,
        out_size,
        "schema=" DENZIC_DEVICE_CONTROL_V1_SETTINGS_REVISION_VALUE_SCHEMA
        ";" DENZIC_DEVICE_CONTROL_V1_SETTINGS_REVISION_VALUE_FIELD "=%" PRIu32,
        revision);
    if (written < 0 || (size_t)written >= out_size) {
        out[0] = '\0';
        return false;
    }
    return true;
}

static bool token_matches(const uint8_t *data, size_t len, const char *token)
{
    size_t token_len;

    if (data == NULL || token == NULL) {
        return false;
    }
    token_len = strlen(token);
    return len == token_len && memcmp(data, token, token_len) == 0;
}

bool denzic_device_control_v1_is_ec11_recovery_notice(
    const uint8_t *data,
    size_t len)
{
    return token_matches(data, len, DENZIC_DEVICE_CONTROL_V1_EC11_RECOVERY_NOTICE);
}

bool denzic_device_control_v1_is_ec11_recovery_prepare_notice(
    const uint8_t *data,
    size_t len)
{
    return token_matches(data, len, DENZIC_DEVICE_CONTROL_V1_EC11_RECOVERY_PREPARE_NOTICE);
}
