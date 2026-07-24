#include "denzic_power_policy_v1.h"

#include <stdio.h>

#define CHECK(condition) do { \
    if (!(condition)) { \
        fprintf(stderr, "CHECK failed at %s:%d: %s\n", __FILE__, __LINE__, #condition); \
        return 1; \
    } \
} while (0)

int main(void)
{
    denzic_power_policy_v1_config_t config = {
        .connected_idle_ms = 100u,
        .disconnected_idle_ms = 200u,
        .shutdown_idle_ms = 1000u,
        .sleep_enabled = true,
        .shutdown_enabled = true,
    };
    denzic_power_policy_v1_input_t input = {
        .radio_idle_ms = 100u,
        .connected = true,
    };
    denzic_power_policy_v1_decision_t decision =
        denzic_power_policy_v1_evaluate(&config, &input);
    CHECK(decision.state == DENZIC_POWER_POLICY_V1_STATE_CONNECTED_IDLE);
    CHECK(decision.sleep_allowed);

    input.sleep_blockers = denzic_power_policy_v1_blocker_bit(
        DENZIC_POWER_POLICY_V1_BLOCKER_RECORDING);
    decision = denzic_power_policy_v1_evaluate(&config, &input);
    CHECK(decision.state == DENZIC_POWER_POLICY_V1_STATE_ACTIVE);

    input.sleep_blockers = 0u;
    input.user_idle_ms = 1000u;
    input.shutdown_blockers = 0u;
    decision = denzic_power_policy_v1_evaluate(&config, &input);
    CHECK(decision.state == DENZIC_POWER_POLICY_V1_STATE_SHUTDOWN_REQUESTED);
    CHECK(decision.shutdown_reason == DENZIC_POWER_POLICY_V1_SHUTDOWN_REASON_LONG_IDLE);

    input.shutdown_blockers = denzic_power_policy_v1_blocker_bit(
        DENZIC_POWER_POLICY_V1_BLOCKER_EXTERNAL_POWER);
    decision = denzic_power_policy_v1_evaluate(&config, &input);
    CHECK(decision.state == DENZIC_POWER_POLICY_V1_STATE_CONNECTED_IDLE);
    return 0;
}
