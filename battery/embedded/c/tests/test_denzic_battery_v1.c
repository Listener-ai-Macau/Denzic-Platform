#include "denzic_battery_v1.h"

#include <stdio.h>

#define CHECK(condition) do { \
    if (!(condition)) { \
        fprintf(stderr, "CHECK failed at %s:%d: %s\n", __FILE__, __LINE__, #condition); \
        return 1; \
    } \
} while (0)

int main(void)
{
    denzic_battery_v1_charge_tracker_t tracker = {0};
    denzic_battery_v1_charge_input_t input = {
        .charge_power_present = true,
        .battery_valid = true,
        .level_percent = 90u,
        .battery_mv = 4100u,
        .raw_full = true,
        .now_ms = 100u,
    };
    denzic_battery_v1_charge_decision_t charge;
    denzic_battery_v1_notify_input_t notify_input = {
        .previous_valid = true,
        .previous_level = 50u,
        .level = 51u,
        .threshold_percent = 1u,
    };
    denzic_battery_v1_notify_decision_t notify;

    CHECK(denzic_battery_v1_percent_from_mv(2850u, 2850u, 4150u) == 0u);
    CHECK(denzic_battery_v1_percent_from_mv(4150u, 2850u, 4150u) == 100u);
    CHECK(denzic_battery_v1_percent_from_mv(3500u, 2850u, 4150u) == 50u);

    charge = denzic_battery_v1_update_charge(&tracker, &input);
    CHECK(!charge.full_latched);
    input.now_ms = 10101u;
    charge = denzic_battery_v1_update_charge(&tracker, &input);
    CHECK(charge.full_latched);
    CHECK(charge.state == DENZIC_BATTERY_V1_CHARGE_STATE_FULL);
    CHECK(charge.published_level == 100u);
    input.charge_power_present = false;
    input.raw_full = false;
    charge = denzic_battery_v1_update_charge(&tracker, &input);
    CHECK(!charge.full_latched);

    notify = denzic_battery_v1_decide_notify(&notify_input);
    CHECK(notify.notify);
    CHECK(notify.reason == DENZIC_BATTERY_V1_NOTIFY_REASON_LEVEL_DELTA);
    notify_input.level = 50u;
    notify_input.now_ms = 60000u;
    notify_input.periodic_interval_ms = 60000u;
    notify = denzic_battery_v1_decide_notify(&notify_input);
    CHECK(notify.notify);
    CHECK(notify.reason == DENZIC_BATTERY_V1_NOTIFY_REASON_PERIODIC);
    return 0;
}
