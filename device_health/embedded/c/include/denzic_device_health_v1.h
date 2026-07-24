#ifndef DENZIC_DEVICE_HEALTH_V1_H
#define DENZIC_DEVICE_HEALTH_V1_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "denzic_device_health_v1_generated.h"

#ifdef __cplusplus
extern "C" {
#endif

typedef struct {
    uint32_t crash_count;
    bool safe_mode_latched;
} denzic_device_health_v1_boot_state_t;

typedef struct {
    denzic_device_health_v1_check_state_t state;
    bool critical;
} denzic_device_health_v1_check_t;

typedef struct {
    uint16_t total;
    uint16_t failed;
    uint16_t critical_failed;
    bool ready;
} denzic_device_health_v1_self_test_summary_t;

typedef struct {
    uint32_t free_heap_kb;
    uint32_t heap_warn_kb;
    uint32_t disconnect_count;
    uint32_t previous_disconnect_count;
    uint32_t disconnect_rate_limit;
} denzic_device_health_v1_runtime_input_t;

typedef struct {
    denzic_device_health_v1_runtime_alert_t alert;
    uint32_t observed;
    uint32_t threshold;
    bool warning;
} denzic_device_health_v1_runtime_decision_t;

bool denzic_device_health_v1_reset_counts_as_crash(
    denzic_device_health_v1_reset_reason_t reason);
void denzic_device_health_v1_boot_observe(
    denzic_device_health_v1_boot_state_t *state,
    denzic_device_health_v1_reset_reason_t reason,
    uint32_t safe_mode_threshold);
void denzic_device_health_v1_boot_clear(
    denzic_device_health_v1_boot_state_t *state);
denzic_device_health_v1_self_test_summary_t
denzic_device_health_v1_summarize_checks(
    const denzic_device_health_v1_check_t *checks,
    size_t count);
denzic_device_health_v1_runtime_decision_t
denzic_device_health_v1_evaluate_runtime(
    const denzic_device_health_v1_runtime_input_t *input);

#ifdef __cplusplus
}
#endif

#endif
