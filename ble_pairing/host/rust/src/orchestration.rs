//! Connection-lifecycle orchestration mirror of the embedded core
//! (`denzic_ble_pairing_v1_orchestration.c`). Same pure decisions: sampled
//! state in, a code out; timers, BLE-stack calls, LED output, persistence,
//! and task plumbing stay in the calling product adapter. Normative semantics
//! live in `ble_pairing/protocol/ble_pairing_v1.md` (section 8).

use crate::generated::{
    ADV_PROFILE_NORMAL, ADV_PROFILE_SWIFT_PAIR, ADV_PROFILE_TYPE_RECOVERY,
    ADV_RESTART_DEFER_BOND_DELETE, ADV_RESTART_RESTART, ADV_RESTART_SUPPRESS_KEY_WAKE,
    ADV_RESTART_SUPPRESS_SHUTDOWN, DISCONNECT_DUPLICATE_FILTER_MS,
};

/// Advertising payload profile (protocol section 8.3).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdvertisingProfile {
    Normal,
    SwiftPair,
    TypeRecovery,
}

impl AdvertisingProfile {
    pub const fn as_u8(self) -> u8 {
        match self {
            Self::Normal => ADV_PROFILE_NORMAL,
            Self::SwiftPair => ADV_PROFILE_SWIFT_PAIR,
            Self::TypeRecovery => ADV_PROFILE_TYPE_RECOVERY,
        }
    }
}

/// Advertising restart routing decision (protocol section 8.1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdvertisingRestart {
    Restart,
    SuppressShutdown,
    SuppressKeyWake,
    DeferBondDelete,
}

impl AdvertisingRestart {
    pub const fn as_u8(self) -> u8 {
        match self {
            Self::Restart => ADV_RESTART_RESTART,
            Self::SuppressShutdown => ADV_RESTART_SUPPRESS_SHUTDOWN,
            Self::SuppressKeyWake => ADV_RESTART_SUPPRESS_KEY_WAKE,
            Self::DeferBondDelete => ADV_RESTART_DEFER_BOND_DELETE,
        }
    }
}

/// Advertising restart routing after a disconnect. Priority: shutdown
/// quiesce, then key-wake-only idle, then an active async bond delete;
/// otherwise restart. The caller samples `shutdown_quiesce` before running
/// its keep-connectable recovery hook and the other inputs after it.
pub const fn adv_restart_after_disconnect(
    shutdown_quiesce: bool,
    key_wake_only: bool,
    bond_delete_active: bool,
) -> AdvertisingRestart {
    if shutdown_quiesce {
        return AdvertisingRestart::SuppressShutdown;
    }
    if key_wake_only {
        return AdvertisingRestart::SuppressKeyWake;
    }
    if bond_delete_active {
        return AdvertisingRestart::DeferBondDelete;
    }
    AdvertisingRestart::Restart
}

/// Advertising restart routing after an advertising-complete event. Same
/// suppress priority, but a bond delete never defers here.
pub const fn adv_restart_after_adv_complete(
    shutdown_quiesce: bool,
    key_wake_only: bool,
) -> AdvertisingRestart {
    if shutdown_quiesce {
        return AdvertisingRestart::SuppressShutdown;
    }
    if key_wake_only {
        return AdvertisingRestart::SuppressKeyWake;
    }
    AdvertisingRestart::Restart
}

/// Duplicate disconnect-event filter: a repeated event for the same
/// connection inside the filter window is a stack echo. The caller compares
/// connection handle and reason itself.
pub const fn disconnect_within_duplicate_filter(previous_event_at_ms: i64, now_ms: i64) -> bool {
    let elapsed_ms = now_ms - previous_event_at_ms;
    elapsed_ms >= 0 && elapsed_ms < DISCONNECT_DUPLICATE_FILTER_MS as i64
}

/// A Type-controlled recovery payload is built first, but only when no Swift
/// Pair prompt competes.
pub const fn adv_profile_try_type_recovery(
    type_recovery_requested: bool,
    swift_pair_requested: bool,
) -> bool {
    type_recovery_requested && !swift_pair_requested
}

/// Swift Pair is tried next; it loses to an already-enabled Type recovery
/// payload.
pub const fn adv_profile_try_swift_pair(
    type_recovery_enabled: bool,
    swift_pair_requested: bool,
) -> bool {
    !type_recovery_enabled && swift_pair_requested
}

/// Final payload profile selection: Swift Pair, then Type recovery, then the
/// normal payload.
pub const fn adv_profile_select(
    type_recovery_enabled: bool,
    swift_pair_enabled: bool,
) -> AdvertisingProfile {
    if swift_pair_enabled {
        return AdvertisingProfile::SwiftPair;
    }
    if type_recovery_enabled {
        return AdvertisingProfile::TypeRecovery;
    }
    AdvertisingProfile::Normal
}

/// Local IRK reset trigger after a recovery disconnect: only inside an open
/// pairing window, only when an identity rotation is pending, and only once a
/// successful bond lookup proves no bonds remain.
pub const fn irk_reset_after_disconnect(
    pairing_window_open: bool,
    identity_rotate_pending: bool,
    bond_lookup_ok: bool,
    bonded_peer_count: i32,
) -> bool {
    pairing_window_open && identity_rotate_pending && bond_lookup_ok && bonded_peer_count == 0
}

/// A bounded non-connectable warm-up advertisement runs before the peer
/// cleanup only for a Type-controlled recovery with a known current peer.
pub const fn bond_delete_warmup_before_delete(type_controlled: bool, known_peer: bool) -> bool {
    type_controlled && known_peer
}

/// Direct single-peer delete without enumeration applies only for a
/// Type-controlled recovery with a known current peer.
pub const fn bond_delete_direct_peer_delete(type_controlled: bool, known_peer: bool) -> bool {
    type_controlled && known_peer
}

/// Per-peer removal during enumeration: a native recovery unpairs through
/// GAP (rotating the local IRK with the final bond); a Type-controlled
/// recovery deletes peer records and keeps the local identity.
pub const fn bond_delete_use_unpair_api(type_controlled: bool) -> bool {
    !type_controlled
}

/// Cleanup success: a direct delete skips the enumeration outcome; an
/// enumeration must have succeeded and no per-peer delete may have failed.
pub const fn bond_delete_cleanup_succeeded(
    fell_back_to_enumeration: bool,
    lookup_rc: i32,
    first_delete_rc: i32,
) -> bool {
    (!fell_back_to_enumeration || lookup_rc == 0) && first_delete_rc == 0
}

/// After the recovery advertising start, the pairing window guard is
/// re-activated only when it lapsed while the window is still open.
pub const fn bond_delete_reactivate_window(
    power_blocker_active: bool,
    pairing_window_active: bool,
) -> bool {
    !power_blocker_active && pairing_window_active
}

/// Passive reattach evidence classification: the host may reclaim the link
/// when Windows already shows a pairing with a live native HID endpoint, or
/// when a fresh native HID address appeared after the monitoring baseline.
/// Address-set diffing stays in the adapter.
pub const fn reattach_evidence_ready(
    paired_devices_visible: bool,
    native_hid_present: bool,
    fresh_native_hid_after_baseline: bool,
) -> bool {
    (paired_devices_visible && native_hid_present) || fresh_native_hid_after_baseline
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn disconnect_restart_priority_order() {
        assert_eq!(
            adv_restart_after_disconnect(false, false, false),
            AdvertisingRestart::Restart
        );
        assert_eq!(
            adv_restart_after_disconnect(true, false, false),
            AdvertisingRestart::SuppressShutdown
        );
        assert_eq!(
            adv_restart_after_disconnect(false, true, false),
            AdvertisingRestart::SuppressKeyWake
        );
        assert_eq!(
            adv_restart_after_disconnect(false, false, true),
            AdvertisingRestart::DeferBondDelete
        );
        assert_eq!(
            adv_restart_after_disconnect(true, true, true),
            AdvertisingRestart::SuppressShutdown
        );
        assert_eq!(
            adv_restart_after_disconnect(false, true, true),
            AdvertisingRestart::SuppressKeyWake
        );
        assert_eq!(
            AdvertisingRestart::DeferBondDelete.as_u8(),
            ADV_RESTART_DEFER_BOND_DELETE
        );
        assert_eq!(AdvertisingRestart::Restart.as_u8(), ADV_RESTART_RESTART);
    }

    #[test]
    fn adv_complete_restart_never_defers() {
        assert_eq!(
            adv_restart_after_adv_complete(false, false),
            AdvertisingRestart::Restart
        );
        assert_eq!(
            adv_restart_after_adv_complete(true, false),
            AdvertisingRestart::SuppressShutdown
        );
        assert_eq!(
            adv_restart_after_adv_complete(false, true),
            AdvertisingRestart::SuppressKeyWake
        );
        assert_eq!(
            adv_restart_after_adv_complete(true, true),
            AdvertisingRestart::SuppressShutdown
        );
    }

    #[test]
    fn duplicate_filter_window_edges() {
        assert!(disconnect_within_duplicate_filter(1000, 1000));
        assert!(disconnect_within_duplicate_filter(1000, 1749));
        assert!(!disconnect_within_duplicate_filter(1000, 1750));
        assert!(!disconnect_within_duplicate_filter(1000, 999));
        assert_eq!(DISCONNECT_DUPLICATE_FILTER_MS, 750);
    }

    #[test]
    fn profile_plan_ordering() {
        assert!(adv_profile_try_type_recovery(true, false));
        assert!(!adv_profile_try_type_recovery(true, true));
        assert!(!adv_profile_try_type_recovery(false, false));
        assert!(adv_profile_try_swift_pair(false, true));
        assert!(!adv_profile_try_swift_pair(true, true));
        assert!(!adv_profile_try_swift_pair(false, false));
        assert_eq!(
            adv_profile_select(false, true),
            AdvertisingProfile::SwiftPair
        );
        assert_eq!(
            adv_profile_select(true, false),
            AdvertisingProfile::TypeRecovery
        );
        assert_eq!(
            adv_profile_select(true, true),
            AdvertisingProfile::SwiftPair
        );
        assert_eq!(adv_profile_select(false, false), AdvertisingProfile::Normal);
        assert_eq!(
            AdvertisingProfile::TypeRecovery.as_u8(),
            ADV_PROFILE_TYPE_RECOVERY
        );
    }

    #[test]
    fn irk_reset_gate() {
        assert!(irk_reset_after_disconnect(true, true, true, 0));
        assert!(!irk_reset_after_disconnect(true, true, true, 1));
        assert!(!irk_reset_after_disconnect(true, true, false, 0));
        assert!(!irk_reset_after_disconnect(false, true, true, 0));
        assert!(!irk_reset_after_disconnect(true, false, true, 0));
    }

    #[test]
    fn bond_delete_sequencing_predicates() {
        assert!(bond_delete_warmup_before_delete(true, true));
        assert!(!bond_delete_warmup_before_delete(true, false));
        assert!(!bond_delete_warmup_before_delete(false, true));
        assert!(bond_delete_direct_peer_delete(true, true));
        assert!(!bond_delete_direct_peer_delete(false, true));
        assert!(bond_delete_use_unpair_api(false));
        assert!(!bond_delete_use_unpair_api(true));
        assert!(bond_delete_cleanup_succeeded(false, -1, 0));
        assert!(!bond_delete_cleanup_succeeded(false, 0, -5));
        assert!(bond_delete_cleanup_succeeded(true, 0, 0));
        assert!(!bond_delete_cleanup_succeeded(true, -1, 0));
        assert!(!bond_delete_cleanup_succeeded(true, 0, -2));
        assert!(bond_delete_reactivate_window(false, true));
        assert!(!bond_delete_reactivate_window(true, true));
        assert!(!bond_delete_reactivate_window(false, false));
    }

    #[test]
    fn reattach_evidence_rows() {
        assert!(reattach_evidence_ready(true, true, false));
        assert!(!reattach_evidence_ready(true, false, false));
        assert!(reattach_evidence_ready(false, false, true));
        assert!(reattach_evidence_ready(false, true, true));
        assert!(!reattach_evidence_ready(false, false, false));
    }
}
