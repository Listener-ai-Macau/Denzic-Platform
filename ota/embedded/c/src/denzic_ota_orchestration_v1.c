#include "denzic_ota_orchestration_v1.h"

denzic_ota_orchestration_v1_blocker_t denzic_ota_orchestration_v1_evaluate_blocker(
    const denzic_ota_orchestration_v1_blocker_inputs_t *inputs,
    uint8_t min_battery_percent)
{
    if (inputs->session_active) {
        return DENZIC_OTA_ORCHESTRATION_V1_BLOCKER_IN_PROGRESS;
    }
    if (inputs->running_pending_verify) {
        return DENZIC_OTA_ORCHESTRATION_V1_BLOCKER_PENDING_VERIFY;
    }
    if (inputs->recording_active) {
        return DENZIC_OTA_ORCHESTRATION_V1_BLOCKER_RECORDING_ACTIVE;
    }
    if (inputs->ble_audio_active) {
        return DENZIC_OTA_ORCHESTRATION_V1_BLOCKER_BLE_AUDIO_ACTIVE;
    }
    if (inputs->diag_export_active) {
        return DENZIC_OTA_ORCHESTRATION_V1_BLOCKER_DIAG_EXPORT_ACTIVE;
    }
    if (inputs->battery_valid && inputs->battery_percent < min_battery_percent) {
        return DENZIC_OTA_ORCHESTRATION_V1_BLOCKER_LOW_BATTERY;
    }
    if (!inputs->has_update_partition) {
        return DENZIC_OTA_ORCHESTRATION_V1_BLOCKER_NO_PARTITION;
    }
    return DENZIC_OTA_ORCHESTRATION_V1_BLOCKER_NONE;
}

bool denzic_ota_orchestration_v1_image_size_accepted(
    size_t image_size,
    size_t unknown_size_sentinel,
    size_t partition_size)
{
    return image_size == 0u || image_size == unknown_size_sentinel ||
           image_size <= partition_size;
}

bool denzic_ota_orchestration_v1_finish_size_matches(
    size_t bytes_written,
    size_t expected_size)
{
    return expected_size == 0u || bytes_written == expected_size;
}

int64_t denzic_ota_orchestration_v1_inactivity_deadline_us(
    int64_t now_us,
    uint32_t timeout_ms)
{
    return now_us + ((int64_t)timeout_ms * 1000LL);
}

bool denzic_ota_orchestration_v1_inactivity_expired(
    int64_t deadline_us,
    int64_t now_us)
{
    return now_us >= deadline_us;
}

denzic_ota_orchestration_v1_pending_decision_t denzic_ota_orchestration_v1_decide_pending_verify(
    bool running_pending_verify,
    bool post_ok,
    bool ble_ready,
    bool keyboard_ready)
{
    denzic_ota_orchestration_v1_pending_decision_t decision = {
        DENZIC_OTA_ORCHESTRATION_V1_PENDING_NONE,
        0u,
    };
    if (!running_pending_verify) {
        return decision;
    }
    if (post_ok && ble_ready && keyboard_ready) {
        decision.action = DENZIC_OTA_ORCHESTRATION_V1_PENDING_CONFIRM;
        return decision;
    }
    decision.action = DENZIC_OTA_ORCHESTRATION_V1_PENDING_ROLLBACK;
    if (!post_ok) {
        decision.rollback_reason_mask |= DENZIC_OTA_ORCHESTRATION_V1_ROLLBACK_POST_FAILED;
    }
    if (!ble_ready) {
        decision.rollback_reason_mask |= DENZIC_OTA_ORCHESTRATION_V1_ROLLBACK_BLE_NOT_READY;
    }
    if (!keyboard_ready) {
        decision.rollback_reason_mask |= DENZIC_OTA_ORCHESTRATION_V1_ROLLBACK_KEYBOARD_NOT_READY;
    }
    return decision;
}
