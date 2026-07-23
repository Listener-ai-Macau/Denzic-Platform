#include "denzic_device_control_v1.h"

#include <assert.h>
#include <string.h>

static denzic_device_control_v1_request_t request(
    uint64_t operation_id,
    uint64_t idempotency_key,
    denzic_device_control_v1_operation_kind_t kind)
{
    denzic_device_control_v1_request_t value = {0};
    value.operation_id = operation_id;
    value.idempotency_key = idempotency_key;
    value.target_id = 17u;
    value.timeout_ms = 20u;
    value.kind = kind;
    return value;
}

int main(void)
{
    denzic_device_control_v1_context_t context;
    denzic_device_control_v1_request_t value;
    denzic_device_control_v1_decision_t outcome;

    denzic_device_control_v1_init(&context, DENZIC_DEVICE_CONTROL_V1_TRANSPORT_BLE);
    denzic_device_control_v1_set_ownership(&context, DENZIC_DEVICE_CONTROL_V1_OWNERSHIP_STATE_LOCAL);

    value = request(1u, 101u, DENZIC_DEVICE_CONTROL_V1_OPERATION_KIND_CONNECT);
    value.automatic = true;
    value.recovery_authorized = true;
    outcome = denzic_device_control_v1_begin(&context, &value, 10u);
    assert(outcome.result == DENZIC_DEVICE_CONTROL_V1_OPERATION_RESULT_ACCEPTED && outcome.execute);
    outcome = denzic_device_control_v1_begin(&context, &value, 11u);
    assert(outcome.result == DENZIC_DEVICE_CONTROL_V1_OPERATION_RESULT_ACCEPTED && !outcome.execute && outcome.replayed);
    denzic_device_control_v1_set_lifecycle(&context, DENZIC_DEVICE_CONTROL_V1_LIFECYCLE_STATE_CONNECTING);
    outcome = denzic_device_control_v1_complete(
        &context,
        1u,
        101u,
        DENZIC_DEVICE_CONTROL_V1_OPERATION_RESULT_SUCCEEDED,
        DENZIC_DEVICE_CONTROL_V1_ERROR_CATEGORY_NONE);
    assert(outcome.result == DENZIC_DEVICE_CONTROL_V1_OPERATION_RESULT_SUCCEEDED);

    value = request(2u, 102u, DENZIC_DEVICE_CONTROL_V1_OPERATION_KIND_SECURE);
    outcome = denzic_device_control_v1_begin(&context, &value, 20u);
    assert(outcome.execute);
    denzic_device_control_v1_set_lifecycle(&context, DENZIC_DEVICE_CONTROL_V1_LIFECYCLE_STATE_SECURING);
    denzic_device_control_v1_complete(
        &context,
        2u,
        102u,
        DENZIC_DEVICE_CONTROL_V1_OPERATION_RESULT_SUCCEEDED,
        DENZIC_DEVICE_CONTROL_V1_ERROR_CATEGORY_NONE);
    denzic_device_control_v1_set_lifecycle(&context, DENZIC_DEVICE_CONTROL_V1_LIFECYCLE_STATE_READY);

    value = request(3u, 103u, DENZIC_DEVICE_CONTROL_V1_OPERATION_KIND_READ_CAPABILITIES);
    assert(denzic_device_control_v1_begin(&context, &value, 30u).execute);
    assert(denzic_device_control_v1_report_capabilities(&context, 3u, 103u, 7u, 0x5u));
    assert(context.capability_revision == 7u && context.capability_mask == 0x5u);

    value = request(4u, 104u, DENZIC_DEVICE_CONTROL_V1_OPERATION_KIND_WRITE_SETTING);
    value.expected_settings_revision = 7u;
    assert(denzic_device_control_v1_begin(&context, &value, 40u).execute);
    outcome = denzic_device_control_v1_complete(
        &context,
        4u,
        104u,
        DENZIC_DEVICE_CONTROL_V1_OPERATION_RESULT_SUCCEEDED,
        DENZIC_DEVICE_CONTROL_V1_ERROR_CATEGORY_NONE);
    assert(outcome.result == DENZIC_DEVICE_CONTROL_V1_OPERATION_RESULT_REJECTED);
    assert(denzic_device_control_v1_acknowledge_setting_write(&context, 4u, 104u, 8u));
    assert(!denzic_device_control_v1_confirm_setting_readback(&context, 4u, 104u, 7u));
    assert(denzic_device_control_v1_confirm_setting_readback(&context, 4u, 104u, 8u));
    outcome = denzic_device_control_v1_begin(&context, &value, 41u);
    assert(outcome.result == DENZIC_DEVICE_CONTROL_V1_OPERATION_RESULT_SUCCEEDED && !outcome.execute && outcome.replayed);

    denzic_device_control_v1_set_ownership(
        &context,
        DENZIC_DEVICE_CONTROL_V1_OWNERSHIP_STATE_MANUAL_UNPAIRED);
    value = request(5u, 105u, DENZIC_DEVICE_CONTROL_V1_OPERATION_KIND_CONNECT);
    value.automatic = true;
    value.recovery_authorized = true;
    outcome = denzic_device_control_v1_begin(&context, &value, 50u);
    assert(outcome.result == DENZIC_DEVICE_CONTROL_V1_OPERATION_RESULT_REJECTED);
    assert(outcome.error == DENZIC_DEVICE_CONTROL_V1_ERROR_CATEGORY_OWNERSHIP);

    denzic_device_control_v1_set_ownership(&context, DENZIC_DEVICE_CONTROL_V1_OWNERSHIP_STATE_LOCAL);
    denzic_device_control_v1_set_lifecycle(&context, DENZIC_DEVICE_CONTROL_V1_LIFECYCLE_STATE_READY);
    value = request(6u, 106u, DENZIC_DEVICE_CONTROL_V1_OPERATION_KIND_INVOKE_COMMAND);
    assert(denzic_device_control_v1_begin(&context, &value, 60u).execute);
    denzic_device_control_v1_set_ownership(&context, DENZIC_DEVICE_CONTROL_V1_OWNERSHIP_STATE_EXTERNAL);
    assert(context.lifecycle == DENZIC_DEVICE_CONTROL_V1_LIFECYCLE_STATE_OWNED_ELSEWHERE);
    outcome = denzic_device_control_v1_begin(&context, &value, 61u);
    assert(outcome.result == DENZIC_DEVICE_CONTROL_V1_OPERATION_RESULT_FAILED && outcome.replayed);

    denzic_device_control_v1_init(&context, DENZIC_DEVICE_CONTROL_V1_TRANSPORT_USB);
    denzic_device_control_v1_set_lifecycle(&context, DENZIC_DEVICE_CONTROL_V1_LIFECYCLE_STATE_READY);
    value = request(7u, 107u, DENZIC_DEVICE_CONTROL_V1_OPERATION_KIND_INVOKE_COMMAND);
    value.timeout_ms = 1u;
    assert(denzic_device_control_v1_begin(&context, &value, 70u).execute);
    assert(denzic_device_control_v1_expire(&context, 71u));
    outcome = denzic_device_control_v1_begin(&context, &value, 72u);
    assert(outcome.result == DENZIC_DEVICE_CONTROL_V1_OPERATION_RESULT_TIMED_OUT && outcome.replayed);

    {
        char revision_value[80];
        char too_small[8];
        static const uint8_t notice[] = "listener-ec11-recovery-v1";
        static const uint8_t prepare_notice[] = "listener-ec11-recovery-prepare-v1";

        assert(denzic_device_control_v1_format_settings_revision(revision_value, sizeof(revision_value), 42u));
        assert(strcmp(revision_value, "schema=listener.device_settings.v1;settings_revision=42") == 0);
        assert(!denzic_device_control_v1_format_settings_revision(too_small, sizeof(too_small), 42u));
        assert(too_small[0] == '\0');
        assert(!denzic_device_control_v1_format_settings_revision(revision_value, sizeof(revision_value), 0u));

        assert(denzic_device_control_v1_is_ec11_recovery_notice(notice, sizeof(notice) - 1u));
        assert(!denzic_device_control_v1_is_ec11_recovery_notice(prepare_notice, sizeof(prepare_notice) - 1u));
        assert(denzic_device_control_v1_is_ec11_recovery_prepare_notice(prepare_notice, sizeof(prepare_notice) - 1u));
        assert(!denzic_device_control_v1_is_ec11_recovery_prepare_notice(notice, sizeof(notice) - 1u));
        assert(!denzic_device_control_v1_is_ec11_recovery_notice(notice, sizeof(notice) - 2u));
    }
    return 0;
}
