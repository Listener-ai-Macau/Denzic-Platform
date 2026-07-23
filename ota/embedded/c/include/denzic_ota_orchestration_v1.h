#ifndef DENZIC_OTA_ORCHESTRATION_V1_H
#define DENZIC_OTA_ORCHESTRATION_V1_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#ifdef __cplusplus
extern "C" {
#endif

/*
 * Product-independent OTA orchestration decisions. The core is a set of pure
 * functions: product adapters sample device facts (activity flags, battery,
 * partition table, monotonic time) and inject them here, then execute the
 * returned decision with their own drivers (partition writes, power rails,
 * timers, LEDs, diagnostics). No ESP-IDF, RTOS, or storage API leaks in.
 */

#define DENZIC_OTA_ORCHESTRATION_V1_MIN_BATTERY_PERCENT (20u)
#define DENZIC_OTA_ORCHESTRATION_V1_INACTIVITY_TIMEOUT_MS (3u * 60u * 1000u)

#define DENZIC_OTA_ORCHESTRATION_V1_ROLLBACK_POST_FAILED (0x01u)
#define DENZIC_OTA_ORCHESTRATION_V1_ROLLBACK_BLE_NOT_READY (0x02u)
#define DENZIC_OTA_ORCHESTRATION_V1_ROLLBACK_KEYBOARD_NOT_READY (0x04u)

/* Values are part of the contract; adapters may map them onto product enums. */
typedef enum {
    DENZIC_OTA_ORCHESTRATION_V1_BLOCKER_NONE = 0,
    DENZIC_OTA_ORCHESTRATION_V1_BLOCKER_IN_PROGRESS = 1,
    DENZIC_OTA_ORCHESTRATION_V1_BLOCKER_PENDING_VERIFY = 2,
    DENZIC_OTA_ORCHESTRATION_V1_BLOCKER_RECORDING_ACTIVE = 3,
    DENZIC_OTA_ORCHESTRATION_V1_BLOCKER_BLE_AUDIO_ACTIVE = 4,
    DENZIC_OTA_ORCHESTRATION_V1_BLOCKER_DIAG_EXPORT_ACTIVE = 5,
    DENZIC_OTA_ORCHESTRATION_V1_BLOCKER_LOW_BATTERY = 6,
    DENZIC_OTA_ORCHESTRATION_V1_BLOCKER_NO_PARTITION = 7
} denzic_ota_orchestration_v1_blocker_t;

typedef struct {
    bool session_active;
    bool running_pending_verify;
    bool recording_active;
    bool ble_audio_active;
    bool diag_export_active;
    bool battery_valid;
    uint8_t battery_percent;
    bool has_update_partition;
} denzic_ota_orchestration_v1_blocker_inputs_t;

/*
 * Evaluates the OTA admission blocker in priority order: an in-progress
 * session wins, then a pending-verify image, then product activity
 * (recording, BLE audio, diagnostic export), then the battery gate, then
 * partition availability. An invalid battery sample never blocks.
 */
denzic_ota_orchestration_v1_blocker_t denzic_ota_orchestration_v1_evaluate_blocker(
    const denzic_ota_orchestration_v1_blocker_inputs_t *inputs,
    uint8_t min_battery_percent);

/*
 * Begin admission: a zero size or the product's unknown-size sentinel always
 * passes; a known image size must fit the target partition.
 */
bool denzic_ota_orchestration_v1_image_size_accepted(
    size_t image_size,
    size_t unknown_size_sentinel,
    size_t partition_size);

/* Finish requires every expected byte; an unknown expected size passes. */
bool denzic_ota_orchestration_v1_finish_size_matches(
    size_t bytes_written,
    size_t expected_size);

/* Inactivity deadline for a stale-session abort, in microseconds. */
int64_t denzic_ota_orchestration_v1_inactivity_deadline_us(
    int64_t now_us,
    uint32_t timeout_ms);

/* True once now_us reaches the deadline; the adapter reschedules otherwise. */
bool denzic_ota_orchestration_v1_inactivity_expired(
    int64_t deadline_us,
    int64_t now_us);

typedef enum {
    DENZIC_OTA_ORCHESTRATION_V1_PENDING_NONE = 0,
    DENZIC_OTA_ORCHESTRATION_V1_PENDING_CONFIRM = 1,
    DENZIC_OTA_ORCHESTRATION_V1_PENDING_ROLLBACK = 2
} denzic_ota_orchestration_v1_pending_action_t;

typedef struct {
    denzic_ota_orchestration_v1_pending_action_t action;
    uint32_t rollback_reason_mask;
} denzic_ota_orchestration_v1_pending_decision_t;

/*
 * Pending-verify confirm/rollback decision. Only a running pending-verify
 * image produces an action; rollback carries one reason bit per failed
 * self-check (POST, BLE readiness, keyboard readiness). The adapter performs
 * the actual mark-valid or mark-invalid-and-reboot call.
 */
denzic_ota_orchestration_v1_pending_decision_t denzic_ota_orchestration_v1_decide_pending_verify(
    bool running_pending_verify,
    bool post_ok,
    bool ble_ready,
    bool keyboard_ready);

#ifdef __cplusplus
}
#endif

#endif
