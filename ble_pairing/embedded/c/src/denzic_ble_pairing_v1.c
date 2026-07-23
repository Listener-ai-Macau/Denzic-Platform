#include "denzic_ble_pairing_v1.h"

uint8_t denzic_ble_pairing_v1_classify_disconnect(
    int32_t reason_code,
    bool locally_requested)
{
    if (locally_requested) {
        return DENZIC_BLE_PAIRING_V1_DISCONNECT_LOCAL_REQUEST;
    }
    if (reason_code == (int32_t)DENZIC_BLE_PAIRING_V1_HCI_HOST_DELIBERATE_DISCONNECT_STATUS) {
        return DENZIC_BLE_PAIRING_V1_DISCONNECT_HOST_DELIBERATE_UNPAIR;
    }
    return DENZIC_BLE_PAIRING_V1_DISCONNECT_TRANSIENT_LINK_LOSS;
}

uint8_t denzic_ble_pairing_v1_advertising_after_disconnect(
    uint8_t classification,
    bool bond_delete_active)
{
    if (bond_delete_active) {
        return DENZIC_BLE_PAIRING_V1_ADVERTISING_AFTER_DISCONNECT_SUPPRESS;
    }
    if (classification == DENZIC_BLE_PAIRING_V1_DISCONNECT_HOST_DELIBERATE_UNPAIR) {
        return DENZIC_BLE_PAIRING_V1_ADVERTISING_AFTER_DISCONNECT_UNDIRECTED_ONLY;
    }
    return DENZIC_BLE_PAIRING_V1_ADVERTISING_AFTER_DISCONNECT_DIRECTED_RECONNECT;
}

bool denzic_ble_pairing_v1_directed_reconnect_allowed(
    bool directed_pending,
    bool low_power,
    bool pairing_window_open)
{
    return directed_pending && !low_power && !pairing_window_open;
}

bool denzic_ble_pairing_v1_advertising_allowed_during_bond_delete(
    bool bond_delete_active,
    bool warmup_permitted)
{
    return bond_delete_active ? warmup_permitted : false;
}

int64_t denzic_ble_pairing_v1_window_remaining_ms(
    int64_t opened_at_ms,
    int64_t window_ms,
    int64_t now_ms)
{
    if (opened_at_ms <= 0 || window_ms <= 0) {
        return 0;
    }

    int64_t elapsed_ms = now_ms - opened_at_ms;
    if (elapsed_ms < 0) {
        return window_ms;
    }
    if (elapsed_ms >= window_ms) {
        return 0;
    }
    return window_ms - elapsed_ms;
}

int32_t denzic_ble_pairing_v1_swift_pair_adv_duration_ms(int64_t remaining_ms)
{
    if (remaining_ms < (int64_t)DENZIC_BLE_PAIRING_V1_SWIFT_PAIR_ADV_MIN_RESTART_MS) {
        return (int32_t)DENZIC_BLE_PAIRING_V1_SWIFT_PAIR_ADV_MIN_RESTART_MS;
    }
    if (remaining_ms > 0x7fffffffLL) {
        return 0x7fffffff;
    }
    return (int32_t)remaining_ms;
}

uint8_t denzic_ble_pairing_v1_evaluate_swift_pair_prompt(
    bool suppressed,
    bool consumed,
    int64_t prompt_window_ms,
    int64_t prompt_remaining_ms)
{
    if (suppressed) {
        return DENZIC_BLE_PAIRING_V1_SWIFT_PAIR_PROMPT_SUPPRESSED;
    }
    if (consumed) {
        return DENZIC_BLE_PAIRING_V1_SWIFT_PAIR_PROMPT_CONSUMED;
    }
    if (prompt_window_ms <= 0) {
        return DENZIC_BLE_PAIRING_V1_SWIFT_PAIR_PROMPT_DISABLED;
    }
    if (prompt_remaining_ms <= 0) {
        return DENZIC_BLE_PAIRING_V1_SWIFT_PAIR_PROMPT_EXPIRED;
    }
    return DENZIC_BLE_PAIRING_V1_SWIFT_PAIR_PROMPT_ACTIVE;
}

uint8_t denzic_ble_pairing_v1_window_close_on_secure(
    bool type_controlled,
    bool window_open)
{
    if (type_controlled && window_open) {
        return DENZIC_BLE_PAIRING_V1_WINDOW_CLOSE_KEEP_OPEN;
    }
    return DENZIC_BLE_PAIRING_V1_WINDOW_CLOSE_CLOSE_NOW;
}

uint8_t denzic_ble_pairing_v1_window_close_on_type_audio_ready(
    bool window_open,
    bool waiting_for_disconnect,
    bool desc_valid,
    bool encrypted,
    bool bonded)
{
    if (window_open &&
        !waiting_for_disconnect &&
        desc_valid &&
        (encrypted || bonded)) {
        return DENZIC_BLE_PAIRING_V1_WINDOW_CLOSE_CLOSE_NOW;
    }
    return DENZIC_BLE_PAIRING_V1_WINDOW_CLOSE_KEEP_OPEN;
}

bool denzic_ble_pairing_v1_type_controlled_recovery(
    bool type_controlled_request,
    bool type_link_ready,
    bool type_host_recent,
    bool connected)
{
    return type_controlled_request ||
           type_link_ready ||
           (type_host_recent && connected);
}

uint8_t denzic_ble_pairing_v1_identity_for_recovery(
    bool type_controlled_recovery,
    bool connected)
{
    if (type_controlled_recovery) {
        return DENZIC_BLE_PAIRING_V1_IDENTITY_KEEP_STABLE;
    }
    if (connected) {
        return DENZIC_BLE_PAIRING_V1_IDENTITY_DEFER_ROTATE_UNTIL_DISCONNECT;
    }
    return DENZIC_BLE_PAIRING_V1_IDENTITY_ROTATE_FRESH;
}

bool denzic_ble_pairing_v1_rotate_before_advertising(bool identity_rotate_pending)
{
    return identity_rotate_pending;
}

bool denzic_ble_pairing_v1_refresh_existing_pairing_window(
    bool window_open,
    int bonded_peer_count,
    bool connected,
    bool force_fresh_identity)
{
    return window_open &&
           bonded_peer_count == 0 &&
           !connected &&
           !force_fresh_identity;
}

uint8_t denzic_ble_pairing_v1_security_failure_action(bool pairing_window_open)
{
    if (pairing_window_open) {
        return DENZIC_BLE_PAIRING_V1_SECURITY_FAILURE_RETRY_WITHIN_WINDOW;
    }
    return DENZIC_BLE_PAIRING_V1_SECURITY_FAILURE_OPEN_REPAIR_WINDOW;
}
