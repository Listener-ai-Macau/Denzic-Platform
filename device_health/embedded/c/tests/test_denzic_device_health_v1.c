#include "denzic_device_health_v1.h"

#include <stdio.h>

#define CHECK(condition) do { \
    if (!(condition)) { \
        fprintf(stderr, "CHECK failed at %s:%d: %s\n", __FILE__, __LINE__, #condition); \
        return 1; \
    } \
} while (0)

int main(void)
{
    denzic_device_health_v1_boot_state_t boot = {0};
    denzic_device_health_v1_check_t checks[3] = {
        {DENZIC_DEVICE_HEALTH_V1_CHECK_STATE_PASSED, true},
        {DENZIC_DEVICE_HEALTH_V1_CHECK_STATE_NOT_REQUIRED, true},
        {DENZIC_DEVICE_HEALTH_V1_CHECK_STATE_RECOVERED, false},
    };
    denzic_device_health_v1_self_test_summary_t summary;
    denzic_device_health_v1_runtime_input_t input = {
        .free_heap_kb = 19u,
        .heap_warn_kb = 20u,
        .disconnect_count = 5u,
        .previous_disconnect_count = 0u,
        .disconnect_rate_limit = 2u,
    };
    denzic_device_health_v1_runtime_decision_t decision;

    denzic_device_health_v1_boot_observe(
        &boot, DENZIC_DEVICE_HEALTH_V1_RESET_REASON_PANIC, 3u);
    denzic_device_health_v1_boot_observe(
        &boot, DENZIC_DEVICE_HEALTH_V1_RESET_REASON_WATCHDOG, 3u);
    CHECK(!boot.safe_mode_latched && boot.crash_count == 2u);
    denzic_device_health_v1_boot_observe(
        &boot, DENZIC_DEVICE_HEALTH_V1_RESET_REASON_SOFTWARE, 3u);
    CHECK(boot.safe_mode_latched && boot.crash_count == 3u);
    denzic_device_health_v1_boot_observe(
        &boot, DENZIC_DEVICE_HEALTH_V1_RESET_REASON_POWER_ON, 3u);
    CHECK(!boot.safe_mode_latched && boot.crash_count == 0u);

    summary = denzic_device_health_v1_summarize_checks(checks, 3u);
    CHECK(summary.ready && summary.total == 3u && summary.failed == 0u);
    checks[1].state = DENZIC_DEVICE_HEALTH_V1_CHECK_STATE_FAILED;
    summary = denzic_device_health_v1_summarize_checks(checks, 3u);
    CHECK(!summary.ready && summary.failed == 1u && summary.critical_failed == 1u);

    decision = denzic_device_health_v1_evaluate_runtime(&input);
    CHECK(decision.alert == DENZIC_DEVICE_HEALTH_V1_RUNTIME_ALERT_HEAP_PRESSURE);
    input.free_heap_kb = 40u;
    decision = denzic_device_health_v1_evaluate_runtime(&input);
    CHECK(decision.alert == DENZIC_DEVICE_HEALTH_V1_RUNTIME_ALERT_LINK_CHURN);
    return 0;
}
