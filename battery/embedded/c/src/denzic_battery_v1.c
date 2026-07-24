#include "denzic_battery_v1.h"

uint8_t denzic_battery_v1_percent_from_mv(
    uint32_t battery_mv,
    uint32_t empty_mv,
    uint32_t full_mv)
{
    uint32_t range_mv;
    uint32_t level;
    if (full_mv <= empty_mv || battery_mv <= empty_mv) {
        return 0u;
    }
    if (battery_mv >= full_mv) {
        return 100u;
    }
    range_mv = full_mv - empty_mv;
    level = ((battery_mv - empty_mv) * 100u + (range_mv / 2u)) / range_mv;
    return (uint8_t)level;
}

denzic_battery_v1_charge_decision_t denzic_battery_v1_update_charge(
    denzic_battery_v1_charge_tracker_t *tracker,
    const denzic_battery_v1_charge_input_t *input)
{
    denzic_battery_v1_charge_decision_t decision = {
        .state = DENZIC_BATTERY_V1_CHARGE_STATE_UNKNOWN,
        .published_level = DENZIC_BATTERY_V1_INVALID_LEVEL,
    };
    bool full_candidate;
    bool battery_allows_full;
    if (tracker == NULL || input == NULL) {
        return decision;
    }
    decision.published_level = input->level_percent > 100u ? 100u : input->level_percent;
    decision.charge_power_present = input->charge_power_present || input->raw_full;
    if (!decision.charge_power_present) {
        tracker->full_latched = false;
        tracker->full_candidate_since_ms = 0u;
        decision.state = DENZIC_BATTERY_V1_CHARGE_STATE_DISCHARGING;
        return decision;
    }

    battery_allows_full =
        !input->battery_valid ||
        input->battery_mv >= DENZIC_BATTERY_V1_CHARGE_FULL_MIN_MV ||
        input->level_percent >= DENZIC_BATTERY_V1_CHARGE_FULL_MIN_PERCENT;
    full_candidate = input->raw_full && !input->raw_charging && battery_allows_full;
    if (!tracker->full_latched) {
        if (!full_candidate) {
            tracker->full_candidate_since_ms = 0u;
        } else if (tracker->full_candidate_since_ms == 0u) {
            tracker->full_candidate_since_ms = input->now_ms;
        } else if (
            input->now_ms - tracker->full_candidate_since_ms >=
            DENZIC_BATTERY_V1_CHARGE_FULL_DEBOUNCE_MS) {
            tracker->full_latched = true;
        }
    }

    decision.full_latched = tracker->full_latched;
    decision.full_candidate_ms =
        tracker->full_candidate_since_ms != 0u
            ? input->now_ms - tracker->full_candidate_since_ms
            : 0u;
    if (tracker->full_latched) {
        decision.state = DENZIC_BATTERY_V1_CHARGE_STATE_FULL;
        decision.published_level = 100u;
    } else if (input->raw_charging) {
        decision.state = DENZIC_BATTERY_V1_CHARGE_STATE_CHARGING;
        if (decision.published_level >= 100u) {
            decision.published_level = 99u;
        }
    } else {
        decision.state = DENZIC_BATTERY_V1_CHARGE_STATE_DISCHARGING;
    }
    return decision;
}

denzic_battery_v1_notify_decision_t denzic_battery_v1_decide_notify(
    const denzic_battery_v1_notify_input_t *input)
{
    denzic_battery_v1_notify_decision_t decision = {
        .reason = DENZIC_BATTERY_V1_NOTIFY_REASON_NONE,
    };
    uint8_t delta;
    if (input == NULL) {
        return decision;
    }
    if (input->force) {
        decision.notify = true;
        decision.reason = DENZIC_BATTERY_V1_NOTIFY_REASON_FORCED;
        return decision;
    }
    if (!input->previous_valid ||
        input->previous_level == DENZIC_BATTERY_V1_INVALID_LEVEL) {
        decision.notify = true;
        decision.reason = DENZIC_BATTERY_V1_NOTIFY_REASON_INITIAL;
        return decision;
    }
    delta = input->level > input->previous_level
        ? (uint8_t)(input->level - input->previous_level)
        : (uint8_t)(input->previous_level - input->level);
    if (delta >= input->threshold_percent) {
        decision.notify = true;
        decision.reason = DENZIC_BATTERY_V1_NOTIFY_REASON_LEVEL_DELTA;
        return decision;
    }
    if (input->periodic_interval_ms > 0u &&
        input->now_ms - input->last_notify_ms >= input->periodic_interval_ms) {
        decision.notify = true;
        decision.reason = DENZIC_BATTERY_V1_NOTIFY_REASON_PERIODIC;
    }
    return decision;
}
