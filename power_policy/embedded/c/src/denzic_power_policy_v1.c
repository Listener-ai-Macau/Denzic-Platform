#include "denzic_power_policy_v1.h"

uint32_t denzic_power_policy_v1_blocker_bit(
    denzic_power_policy_v1_blocker_t blocker)
{
    uint32_t index = (uint32_t)blocker;
    return index <= DENZIC_POWER_POLICY_V1_MAX_BLOCKER_INDEX
        ? (1u << index)
        : 0u;
}

denzic_power_policy_v1_decision_t denzic_power_policy_v1_evaluate(
    const denzic_power_policy_v1_config_t *config,
    const denzic_power_policy_v1_input_t *input)
{
    denzic_power_policy_v1_decision_t decision = {
        .state = DENZIC_POWER_POLICY_V1_STATE_ACTIVE,
        .shutdown_reason = DENZIC_POWER_POLICY_V1_SHUTDOWN_REASON_NONE,
    };
    uint32_t idle_threshold;
    if (config == NULL || input == NULL) {
        return decision;
    }
    if (config->shutdown_enabled &&
        config->shutdown_idle_ms > 0u &&
        input->user_idle_ms >= config->shutdown_idle_ms &&
        input->shutdown_blockers == 0u &&
        !input->shutdown_retry_active) {
        decision.state = DENZIC_POWER_POLICY_V1_STATE_SHUTDOWN_REQUESTED;
        decision.shutdown_reason = DENZIC_POWER_POLICY_V1_SHUTDOWN_REASON_LONG_IDLE;
        decision.shutdown_allowed = true;
        return decision;
    }
    if (!config->sleep_enabled || input->sleep_blockers != 0u) {
        return decision;
    }
    idle_threshold = input->connected
        ? config->connected_idle_ms
        : config->disconnected_idle_ms;
    if (idle_threshold == 0u || input->radio_idle_ms < idle_threshold) {
        return decision;
    }
    decision.sleep_allowed = true;
    decision.state = input->connected
        ? DENZIC_POWER_POLICY_V1_STATE_CONNECTED_IDLE
        : DENZIC_POWER_POLICY_V1_STATE_DISCONNECTED_IDLE;
    return decision;
}
