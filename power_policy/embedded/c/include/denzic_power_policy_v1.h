#ifndef DENZIC_POWER_POLICY_V1_H
#define DENZIC_POWER_POLICY_V1_H

#include <stdbool.h>
#include <stdint.h>

#include "denzic_power_policy_v1_generated.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
    uint32_t connected_idle_ms;
    uint32_t disconnected_idle_ms;
    uint32_t shutdown_idle_ms;
    bool sleep_enabled;
    bool shutdown_enabled;
} denzic_power_policy_v1_config_t;

typedef struct {
    uint32_t sleep_blockers;
    uint32_t shutdown_blockers;
    uint32_t user_idle_ms;
    uint32_t radio_idle_ms;
    bool connected;
    bool shutdown_retry_active;
} denzic_power_policy_v1_input_t;

typedef struct {
    denzic_power_policy_v1_state_t state;
    denzic_power_policy_v1_shutdown_reason_t shutdown_reason;
    bool sleep_allowed;
    bool shutdown_allowed;
} denzic_power_policy_v1_decision_t;

uint32_t denzic_power_policy_v1_blocker_bit(
    denzic_power_policy_v1_blocker_t blocker);
denzic_power_policy_v1_decision_t denzic_power_policy_v1_evaluate(
    const denzic_power_policy_v1_config_t *config,
    const denzic_power_policy_v1_input_t *input);

#ifdef __cplusplus
}
#endif

#endif
