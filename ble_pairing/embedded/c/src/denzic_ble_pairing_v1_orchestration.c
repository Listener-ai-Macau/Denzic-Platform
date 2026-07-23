#include "denzic_ble_pairing_v1_orchestration.h"

uint8_t denzic_ble_pairing_v1_orch_adv_restart_after_disconnect(
    bool shutdown_quiesce,
    bool key_wake_only,
    bool bond_delete_active)
{
    if (shutdown_quiesce) {
        return DENZIC_BLE_PAIRING_V1_ADV_RESTART_SUPPRESS_SHUTDOWN;
    }
    if (key_wake_only) {
        return DENZIC_BLE_PAIRING_V1_ADV_RESTART_SUPPRESS_KEY_WAKE;
    }
    if (bond_delete_active) {
        return DENZIC_BLE_PAIRING_V1_ADV_RESTART_DEFER_BOND_DELETE;
    }
    return DENZIC_BLE_PAIRING_V1_ADV_RESTART_RESTART;
}

uint8_t denzic_ble_pairing_v1_orch_adv_restart_after_adv_complete(
    bool shutdown_quiesce,
    bool key_wake_only)
{
    if (shutdown_quiesce) {
        return DENZIC_BLE_PAIRING_V1_ADV_RESTART_SUPPRESS_SHUTDOWN;
    }
    if (key_wake_only) {
        return DENZIC_BLE_PAIRING_V1_ADV_RESTART_SUPPRESS_KEY_WAKE;
    }
    return DENZIC_BLE_PAIRING_V1_ADV_RESTART_RESTART;
}

bool denzic_ble_pairing_v1_orch_disconnect_within_duplicate_filter(
    int64_t previous_event_at_ms,
    int64_t now_ms)
{
    int64_t elapsed_ms = now_ms - previous_event_at_ms;
    return elapsed_ms >= 0 &&
           elapsed_ms < (int64_t)DENZIC_BLE_PAIRING_V1_DISCONNECT_DUPLICATE_FILTER_MS;
}

bool denzic_ble_pairing_v1_orch_adv_profile_try_type_recovery(
    bool type_recovery_requested,
    bool swift_pair_requested)
{
    return type_recovery_requested && !swift_pair_requested;
}

bool denzic_ble_pairing_v1_orch_adv_profile_try_swift_pair(
    bool type_recovery_enabled,
    bool swift_pair_requested)
{
    return !type_recovery_enabled && swift_pair_requested;
}

uint8_t denzic_ble_pairing_v1_orch_adv_profile_select(
    bool type_recovery_enabled,
    bool swift_pair_enabled)
{
    if (swift_pair_enabled) {
        return DENZIC_BLE_PAIRING_V1_ADV_PROFILE_SWIFT_PAIR;
    }
    if (type_recovery_enabled) {
        return DENZIC_BLE_PAIRING_V1_ADV_PROFILE_TYPE_RECOVERY;
    }
    return DENZIC_BLE_PAIRING_V1_ADV_PROFILE_NORMAL;
}

bool denzic_ble_pairing_v1_orch_irk_reset_after_disconnect(
    bool pairing_window_open,
    bool identity_rotate_pending,
    bool bond_lookup_ok,
    int bonded_peer_count)
{
    return pairing_window_open && identity_rotate_pending && bond_lookup_ok &&
           bonded_peer_count == 0;
}

bool denzic_ble_pairing_v1_orch_bond_delete_warmup_before_delete(
    bool type_controlled,
    bool known_peer)
{
    return type_controlled && known_peer;
}

bool denzic_ble_pairing_v1_orch_bond_delete_direct_peer_delete(
    bool type_controlled,
    bool known_peer)
{
    return type_controlled && known_peer;
}

bool denzic_ble_pairing_v1_orch_bond_delete_use_unpair_api(bool type_controlled)
{
    return !type_controlled;
}

bool denzic_ble_pairing_v1_orch_bond_delete_cleanup_succeeded(
    bool fell_back_to_enumeration,
    int lookup_rc,
    int first_delete_rc)
{
    return (!fell_back_to_enumeration || lookup_rc == 0) && first_delete_rc == 0;
}

bool denzic_ble_pairing_v1_orch_bond_delete_reactivate_window(
    bool power_blocker_active,
    bool pairing_window_active)
{
    return !power_blocker_active && pairing_window_active;
}

bool denzic_ble_pairing_v1_orch_reattach_evidence_ready(
    bool paired_devices_visible,
    bool native_hid_present,
    bool fresh_native_hid_after_baseline)
{
    return (paired_devices_visible && native_hid_present) ||
           fresh_native_hid_after_baseline;
}
