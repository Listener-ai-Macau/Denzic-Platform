#ifndef DENZIC_BLE_PAIRING_V1_H
#define DENZIC_BLE_PAIRING_V1_H

#include <stdbool.h>
#include <stdint.h>

#include "denzic_ble_pairing_v1_generated.h"

/*
 * Portable BLE pairing/recovery decision core. Every function is a pure
 * decision table: numbers and booleans in, a code out. The caller keeps all
 * OS, timer, NVS, and BLE-stack side effects. Normative semantics live in
 * ble_pairing/protocol/ble_pairing_v1.md.
 */

#ifdef __cplusplus
extern "C" {
#endif

/* Disconnect classification (protocol section 1). reason_code is the host
 * stack disconnect status (HCI error already composed); locally_requested
 * comes from the caller's own terminate bookkeeping. */
uint8_t denzic_ble_pairing_v1_classify_disconnect(
    int32_t reason_code,
    bool locally_requested);

/* Advertising posture after a disconnect (protocol section 2, no-chase rule). */
uint8_t denzic_ble_pairing_v1_advertising_after_disconnect(
    uint8_t classification,
    bool bond_delete_active);

/* Extra gate applied at advertising start (protocol section 2). A bonded
 * peer address must also exist; the caller checks that before asking. */
bool denzic_ble_pairing_v1_directed_reconnect_allowed(
    bool directed_pending,
    bool low_power,
    bool pairing_window_open);

/* Bond-delete suppression (protocol section 3). */
bool denzic_ble_pairing_v1_advertising_allowed_during_bond_delete(
    bool bond_delete_active,
    bool warmup_permitted);

/* Window arithmetic (protocol section 4). */
int64_t denzic_ble_pairing_v1_window_remaining_ms(
    int64_t opened_at_ms,
    int64_t window_ms,
    int64_t now_ms);

/* Bounded Swift Pair advertisement duration clamp (protocol section 5). */
int32_t denzic_ble_pairing_v1_swift_pair_adv_duration_ms(int64_t remaining_ms);

/* Single-shot Swift Pair prompt evaluation (protocol section 5). */
uint8_t denzic_ble_pairing_v1_evaluate_swift_pair_prompt(
    bool suppressed,
    bool consumed,
    int64_t prompt_window_ms,
    int64_t prompt_remaining_ms);

/* Secure-connection window decision (protocol section 4). */
uint8_t denzic_ble_pairing_v1_window_close_on_secure(
    bool type_controlled,
    bool window_open);

/* Type audio-ready window decision (protocol section 4). */
uint8_t denzic_ble_pairing_v1_window_close_on_type_audio_ready(
    bool window_open,
    bool waiting_for_disconnect,
    bool desc_valid,
    bool encrypted,
    bool bonded);

/* Type-controlled recovery ownership test (protocol section 6). */
bool denzic_ble_pairing_v1_type_controlled_recovery(
    bool type_controlled_request,
    bool type_link_ready,
    bool type_host_recent,
    bool connected);

/* Identity action for a recovery pairing reset (protocol section 6). */
uint8_t denzic_ble_pairing_v1_identity_for_recovery(
    bool type_controlled_recovery,
    bool connected);

/* Deferred rotation ordering (protocol section 6): rotation runs after the
 * disconnect/bond cleanup and before the recovery advertising start. */
bool denzic_ble_pairing_v1_rotate_before_advertising(bool identity_rotate_pending);

/* In-place refresh of an already-open window (protocol section 6). */
bool denzic_ble_pairing_v1_refresh_existing_pairing_window(
    bool window_open,
    int bonded_peer_count,
    bool connected,
    bool force_fresh_identity);

/* Security-failure repair decision (protocol section 7). */
uint8_t denzic_ble_pairing_v1_security_failure_action(bool pairing_window_open);

#ifdef __cplusplus
}
#endif

#endif
