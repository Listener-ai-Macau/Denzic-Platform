#ifndef DENZIC_BATTERY_V1_H
#define DENZIC_BATTERY_V1_H

#include <stdbool.h>
#include <stdint.h>

#include "denzic_battery_v1_generated.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
    bool full_latched;
    uint32_t full_candidate_since_ms;
} denzic_battery_v1_charge_tracker_t;

typedef struct {
    bool charge_power_present;
    bool battery_valid;
    uint8_t level_percent;
    uint32_t battery_mv;
    bool raw_charging;
    bool raw_full;
    uint32_t now_ms;
} denzic_battery_v1_charge_input_t;

typedef struct {
    denzic_battery_v1_charge_state_t state;
    uint8_t published_level;
    bool charge_power_present;
    bool full_latched;
    uint32_t full_candidate_ms;
} denzic_battery_v1_charge_decision_t;

typedef struct {
    bool previous_valid;
    uint8_t previous_level;
    uint8_t level;
    bool force;
    uint32_t now_ms;
    uint32_t last_notify_ms;
    uint32_t periodic_interval_ms;
    uint8_t threshold_percent;
} denzic_battery_v1_notify_input_t;

typedef struct {
    bool notify;
    denzic_battery_v1_notify_reason_t reason;
} denzic_battery_v1_notify_decision_t;

uint8_t denzic_battery_v1_percent_from_mv(
    uint32_t battery_mv,
    uint32_t empty_mv,
    uint32_t full_mv);
denzic_battery_v1_charge_decision_t denzic_battery_v1_update_charge(
    denzic_battery_v1_charge_tracker_t *tracker,
    const denzic_battery_v1_charge_input_t *input);
denzic_battery_v1_notify_decision_t denzic_battery_v1_decide_notify(
    const denzic_battery_v1_notify_input_t *input);

#ifdef __cplusplus
}
#endif

#endif
