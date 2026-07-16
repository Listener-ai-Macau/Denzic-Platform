#ifndef DENZIC_DEVICE_CONTROL_V1_H
#define DENZIC_DEVICE_CONTROL_V1_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "denzic_device_control_v1_generated.h"

#ifdef __cplusplus
extern "C" {
#endif

#define DENZIC_DEVICE_CONTROL_V1_TERMINAL_HISTORY_CAPACITY (4u)

typedef struct {
    uint64_t operation_id;
    uint64_t idempotency_key;
    uint32_t target_id;
    uint32_t expected_settings_revision;
    uint32_t timeout_ms;
    denzic_device_control_v1_operation_kind_t kind;
    bool automatic;
    bool recovery_authorized;
} denzic_device_control_v1_request_t;

typedef struct {
    uint64_t operation_id;
    uint64_t idempotency_key;
    denzic_device_control_v1_operation_kind_t kind;
    denzic_device_control_v1_operation_result_t result;
    denzic_device_control_v1_error_category_t error;
    bool valid;
} denzic_device_control_v1_terminal_record_t;

typedef struct {
    denzic_device_control_v1_operation_result_t result;
    denzic_device_control_v1_error_category_t error;
    bool execute;
    bool replayed;
} denzic_device_control_v1_decision_t;

typedef struct {
    denzic_device_control_v1_transport_t transport;
    denzic_device_control_v1_lifecycle_state_t lifecycle;
    denzic_device_control_v1_ownership_state_t ownership;
    uint64_t capability_mask;
    uint32_t capability_revision;
    uint32_t settings_revision;
    bool has_active_operation;
    denzic_device_control_v1_request_t active_request;
    uint32_t active_deadline_ms;
    bool active_write_acknowledged;
    uint32_t active_write_acknowledged_revision;
    denzic_device_control_v1_terminal_record_t
        terminal_history[DENZIC_DEVICE_CONTROL_V1_TERMINAL_HISTORY_CAPACITY];
    size_t terminal_history_next;
} denzic_device_control_v1_context_t;

void denzic_device_control_v1_init(
    denzic_device_control_v1_context_t *context,
    denzic_device_control_v1_transport_t transport);

void denzic_device_control_v1_set_lifecycle(
    denzic_device_control_v1_context_t *context,
    denzic_device_control_v1_lifecycle_state_t lifecycle);

void denzic_device_control_v1_set_ownership(
    denzic_device_control_v1_context_t *context,
    denzic_device_control_v1_ownership_state_t ownership);

denzic_device_control_v1_decision_t denzic_device_control_v1_begin(
    denzic_device_control_v1_context_t *context,
    const denzic_device_control_v1_request_t *request,
    uint32_t now_ms);

denzic_device_control_v1_decision_t denzic_device_control_v1_cancel(
    denzic_device_control_v1_context_t *context,
    uint64_t operation_id,
    uint64_t idempotency_key);

denzic_device_control_v1_decision_t denzic_device_control_v1_complete(
    denzic_device_control_v1_context_t *context,
    uint64_t operation_id,
    uint64_t idempotency_key,
    denzic_device_control_v1_operation_result_t result,
    denzic_device_control_v1_error_category_t error);

bool denzic_device_control_v1_expire(
    denzic_device_control_v1_context_t *context,
    uint32_t now_ms);

bool denzic_device_control_v1_report_transport_lost(
    denzic_device_control_v1_context_t *context);

bool denzic_device_control_v1_report_capabilities(
    denzic_device_control_v1_context_t *context,
    uint64_t operation_id,
    uint64_t idempotency_key,
    uint32_t revision,
    uint64_t capability_mask);

bool denzic_device_control_v1_acknowledge_setting_write(
    denzic_device_control_v1_context_t *context,
    uint64_t operation_id,
    uint64_t idempotency_key,
    uint32_t acknowledged_revision);

bool denzic_device_control_v1_confirm_setting_readback(
    denzic_device_control_v1_context_t *context,
    uint64_t operation_id,
    uint64_t idempotency_key,
    uint32_t observed_revision);

#ifdef __cplusplus
}
#endif

#endif
