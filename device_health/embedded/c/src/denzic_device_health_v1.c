#include "denzic_device_health_v1.h"

#include <limits.h>

bool denzic_device_health_v1_reset_counts_as_crash(
    denzic_device_health_v1_reset_reason_t reason)
{
    switch (reason) {
    case DENZIC_DEVICE_HEALTH_V1_RESET_REASON_POWER_ON:
    case DENZIC_DEVICE_HEALTH_V1_RESET_REASON_EXTERNAL:
    case DENZIC_DEVICE_HEALTH_V1_RESET_REASON_DEEP_SLEEP:
    case DENZIC_DEVICE_HEALTH_V1_RESET_REASON_USB:
    case DENZIC_DEVICE_HEALTH_V1_RESET_REASON_JTAG:
        return false;
    default:
        return true;
    }
}

void denzic_device_health_v1_boot_observe(
    denzic_device_health_v1_boot_state_t *state,
    denzic_device_health_v1_reset_reason_t reason,
    uint32_t safe_mode_threshold)
{
    if (state == NULL) {
        return;
    }
    if (!denzic_device_health_v1_reset_counts_as_crash(reason)) {
        denzic_device_health_v1_boot_clear(state);
        return;
    }
    if (state->crash_count < UINT32_MAX) {
        state->crash_count++;
    }
    if (safe_mode_threshold > 0u && state->crash_count >= safe_mode_threshold) {
        state->safe_mode_latched = true;
    }
}

void denzic_device_health_v1_boot_clear(
    denzic_device_health_v1_boot_state_t *state)
{
    if (state != NULL) {
        state->crash_count = 0u;
        state->safe_mode_latched = false;
    }
}

denzic_device_health_v1_self_test_summary_t
denzic_device_health_v1_summarize_checks(
    const denzic_device_health_v1_check_t *checks,
    size_t count)
{
    denzic_device_health_v1_self_test_summary_t summary = {0};
    size_t index;
    if (checks == NULL && count != 0u) {
        summary.ready = false;
        summary.critical_failed = 1u;
        return summary;
    }
    summary.ready = true;
    for (index = 0u; index < count; index++) {
        bool failed = checks[index].state == DENZIC_DEVICE_HEALTH_V1_CHECK_STATE_FAILED;
        if (summary.total < UINT16_MAX) {
            summary.total++;
        }
        if (failed && summary.failed < UINT16_MAX) {
            summary.failed++;
        }
        if (failed && checks[index].critical && summary.critical_failed < UINT16_MAX) {
            summary.critical_failed++;
            summary.ready = false;
        }
    }
    return summary;
}

denzic_device_health_v1_runtime_decision_t
denzic_device_health_v1_evaluate_runtime(
    const denzic_device_health_v1_runtime_input_t *input)
{
    denzic_device_health_v1_runtime_decision_t decision = {
        .alert = DENZIC_DEVICE_HEALTH_V1_RUNTIME_ALERT_NONE,
    };
    uint32_t disconnect_delta;
    if (input == NULL) {
        return decision;
    }
    if (input->heap_warn_kb > 0u && input->free_heap_kb < input->heap_warn_kb) {
        decision.alert = DENZIC_DEVICE_HEALTH_V1_RUNTIME_ALERT_HEAP_PRESSURE;
        decision.observed = input->free_heap_kb;
        decision.threshold = input->heap_warn_kb;
        decision.warning = true;
        return decision;
    }
    disconnect_delta = input->disconnect_count - input->previous_disconnect_count;
    if (disconnect_delta > input->disconnect_rate_limit) {
        decision.alert = DENZIC_DEVICE_HEALTH_V1_RUNTIME_ALERT_LINK_CHURN;
        decision.observed = disconnect_delta;
        decision.threshold = input->disconnect_rate_limit;
        decision.warning = true;
    }
    return decision;
}
