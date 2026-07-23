#include "denzic_ota_orchestration_v1.h"

#include <assert.h>

static void test_blocker_priority(void)
{
    denzic_ota_orchestration_v1_blocker_inputs_t inputs = {
        false, false, false, false, false, true, 100u, true,
    };

    assert(denzic_ota_orchestration_v1_evaluate_blocker(
               &inputs, DENZIC_OTA_ORCHESTRATION_V1_MIN_BATTERY_PERCENT) ==
           DENZIC_OTA_ORCHESTRATION_V1_BLOCKER_NONE);

    inputs.battery_percent = 19u;
    assert(denzic_ota_orchestration_v1_evaluate_blocker(
               &inputs, DENZIC_OTA_ORCHESTRATION_V1_MIN_BATTERY_PERCENT) ==
           DENZIC_OTA_ORCHESTRATION_V1_BLOCKER_LOW_BATTERY);

    /* An invalid battery sample never blocks on its own. */
    inputs.battery_valid = false;
    assert(denzic_ota_orchestration_v1_evaluate_blocker(
               &inputs, DENZIC_OTA_ORCHESTRATION_V1_MIN_BATTERY_PERCENT) ==
           DENZIC_OTA_ORCHESTRATION_V1_BLOCKER_NONE);

    /* Low battery outranks a missing update partition. */
    inputs.battery_valid = true;
    inputs.has_update_partition = false;
    assert(denzic_ota_orchestration_v1_evaluate_blocker(
               &inputs, DENZIC_OTA_ORCHESTRATION_V1_MIN_BATTERY_PERCENT) ==
           DENZIC_OTA_ORCHESTRATION_V1_BLOCKER_LOW_BATTERY);
    inputs.battery_percent = 100u;
    assert(denzic_ota_orchestration_v1_evaluate_blocker(
               &inputs, DENZIC_OTA_ORCHESTRATION_V1_MIN_BATTERY_PERCENT) ==
           DENZIC_OTA_ORCHESTRATION_V1_BLOCKER_NO_PARTITION);
    inputs.has_update_partition = true;

    /* Activity flags outrank the battery gate; session state wins overall. */
    inputs.battery_percent = 0u;
    inputs.diag_export_active = true;
    assert(denzic_ota_orchestration_v1_evaluate_blocker(
               &inputs, DENZIC_OTA_ORCHESTRATION_V1_MIN_BATTERY_PERCENT) ==
           DENZIC_OTA_ORCHESTRATION_V1_BLOCKER_DIAG_EXPORT_ACTIVE);
    inputs.ble_audio_active = true;
    assert(denzic_ota_orchestration_v1_evaluate_blocker(
               &inputs, DENZIC_OTA_ORCHESTRATION_V1_MIN_BATTERY_PERCENT) ==
           DENZIC_OTA_ORCHESTRATION_V1_BLOCKER_BLE_AUDIO_ACTIVE);
    inputs.recording_active = true;
    assert(denzic_ota_orchestration_v1_evaluate_blocker(
               &inputs, DENZIC_OTA_ORCHESTRATION_V1_MIN_BATTERY_PERCENT) ==
           DENZIC_OTA_ORCHESTRATION_V1_BLOCKER_RECORDING_ACTIVE);
    inputs.running_pending_verify = true;
    assert(denzic_ota_orchestration_v1_evaluate_blocker(
               &inputs, DENZIC_OTA_ORCHESTRATION_V1_MIN_BATTERY_PERCENT) ==
           DENZIC_OTA_ORCHESTRATION_V1_BLOCKER_PENDING_VERIFY);
    inputs.session_active = true;
    assert(denzic_ota_orchestration_v1_evaluate_blocker(
               &inputs, DENZIC_OTA_ORCHESTRATION_V1_MIN_BATTERY_PERCENT) ==
           DENZIC_OTA_ORCHESTRATION_V1_BLOCKER_IN_PROGRESS);

    /* The battery gate boundary is inclusive: exactly the threshold passes. */
    inputs.session_active = false;
    inputs.running_pending_verify = false;
    inputs.recording_active = false;
    inputs.ble_audio_active = false;
    inputs.diag_export_active = false;
    inputs.battery_percent = DENZIC_OTA_ORCHESTRATION_V1_MIN_BATTERY_PERCENT;
    assert(denzic_ota_orchestration_v1_evaluate_blocker(
               &inputs, DENZIC_OTA_ORCHESTRATION_V1_MIN_BATTERY_PERCENT) ==
           DENZIC_OTA_ORCHESTRATION_V1_BLOCKER_NONE);
}

static void test_image_size_admission(void)
{
    const size_t unknown = (size_t)0xffffffffu;

    assert(denzic_ota_orchestration_v1_image_size_accepted(0u, unknown, 1024u));
    assert(denzic_ota_orchestration_v1_image_size_accepted(unknown, unknown, 1024u));
    assert(denzic_ota_orchestration_v1_image_size_accepted(1024u, unknown, 1024u));
    assert(!denzic_ota_orchestration_v1_image_size_accepted(1025u, unknown, 1024u));
}

static void test_finish_size_rule(void)
{
    assert(denzic_ota_orchestration_v1_finish_size_matches(0u, 0u));
    assert(denzic_ota_orchestration_v1_finish_size_matches(64u, 64u));
    assert(!denzic_ota_orchestration_v1_finish_size_matches(63u, 64u));
}

static void test_inactivity_rules(void)
{
    const int64_t deadline = denzic_ota_orchestration_v1_inactivity_deadline_us(
        1000000LL, DENZIC_OTA_ORCHESTRATION_V1_INACTIVITY_TIMEOUT_MS);

    assert(deadline == 1000000LL + 180000000LL);
    assert(!denzic_ota_orchestration_v1_inactivity_expired(deadline, deadline - 1));
    assert(denzic_ota_orchestration_v1_inactivity_expired(deadline, deadline));
}

static void test_pending_verify_decision(void)
{
    denzic_ota_orchestration_v1_pending_decision_t decision;

    decision = denzic_ota_orchestration_v1_decide_pending_verify(false, false, false, false);
    assert(decision.action == DENZIC_OTA_ORCHESTRATION_V1_PENDING_NONE);
    assert(decision.rollback_reason_mask == 0u);

    decision = denzic_ota_orchestration_v1_decide_pending_verify(true, true, true, true);
    assert(decision.action == DENZIC_OTA_ORCHESTRATION_V1_PENDING_CONFIRM);
    assert(decision.rollback_reason_mask == 0u);

    decision = denzic_ota_orchestration_v1_decide_pending_verify(true, false, true, false);
    assert(decision.action == DENZIC_OTA_ORCHESTRATION_V1_PENDING_ROLLBACK);
    assert(decision.rollback_reason_mask ==
           (DENZIC_OTA_ORCHESTRATION_V1_ROLLBACK_POST_FAILED |
            DENZIC_OTA_ORCHESTRATION_V1_ROLLBACK_KEYBOARD_NOT_READY));

    decision = denzic_ota_orchestration_v1_decide_pending_verify(true, true, false, true);
    assert(decision.action == DENZIC_OTA_ORCHESTRATION_V1_PENDING_ROLLBACK);
    assert(decision.rollback_reason_mask == DENZIC_OTA_ORCHESTRATION_V1_ROLLBACK_BLE_NOT_READY);
}

int main(void)
{
    test_blocker_priority();
    test_image_size_admission();
    test_finish_size_rule();
    test_inactivity_rules();
    test_pending_verify_decision();
    return 0;
}
