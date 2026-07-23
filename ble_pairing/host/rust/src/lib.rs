//! Product-agnostic BLE pairing/recovery decision mirror.
//!
//! Pure decision tables shared with the embedded core
//! (`denzic_ble_pairing_v1.c`); numbers and booleans in, a code out. Device
//! names, window durations beyond the defaults, and every side effect
//! (advertising, identity rotation, persistence, timers, logging) stay in the
//! calling product adapter. Normative semantics live in
//! `ble_pairing/protocol/ble_pairing_v1.md`.

mod generated;

pub use generated::*;

/// Disconnect classification (protocol section 1).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DisconnectClassification {
    TransientLinkLoss,
    HostDeliberateUnpair,
    LocalRequest,
}

impl DisconnectClassification {
    pub const fn as_u8(self) -> u8 {
        match self {
            Self::TransientLinkLoss => DISCONNECT_TRANSIENT_LINK_LOSS,
            Self::HostDeliberateUnpair => DISCONNECT_HOST_DELIBERATE_UNPAIR,
            Self::LocalRequest => DISCONNECT_LOCAL_REQUEST,
        }
    }
}

/// Advertising posture after a disconnect (protocol section 2).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdvertisingAfterDisconnect {
    Suppress,
    UndirectedOnly,
    DirectedReconnect,
}

impl AdvertisingAfterDisconnect {
    pub const fn as_u8(self) -> u8 {
        match self {
            Self::Suppress => ADVERTISING_AFTER_DISCONNECT_SUPPRESS,
            Self::UndirectedOnly => ADVERTISING_AFTER_DISCONNECT_UNDIRECTED_ONLY,
            Self::DirectedReconnect => ADVERTISING_AFTER_DISCONNECT_DIRECTED_RECONNECT,
        }
    }
}

/// Identity action for a recovery pairing reset (protocol section 6).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityAction {
    KeepStable,
    RotateFresh,
    DeferRotateUntilDisconnect,
}

impl IdentityAction {
    pub const fn as_u8(self) -> u8 {
        match self {
            Self::KeepStable => IDENTITY_KEEP_STABLE,
            Self::RotateFresh => IDENTITY_ROTATE_FRESH,
            Self::DeferRotateUntilDisconnect => IDENTITY_DEFER_ROTATE_UNTIL_DISCONNECT,
        }
    }
}

/// Window close decision (protocol section 4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WindowCloseDecision {
    KeepOpen,
    CloseNow,
}

impl WindowCloseDecision {
    pub const fn as_u8(self) -> u8 {
        match self {
            Self::KeepOpen => WINDOW_CLOSE_KEEP_OPEN,
            Self::CloseNow => WINDOW_CLOSE_CLOSE_NOW,
        }
    }
}

/// Security-failure repair decision (protocol section 7).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SecurityFailureAction {
    RetryWithinWindow,
    OpenRepairWindow,
}

impl SecurityFailureAction {
    pub const fn as_u8(self) -> u8 {
        match self {
            Self::RetryWithinWindow => SECURITY_FAILURE_RETRY_WITHIN_WINDOW,
            Self::OpenRepairWindow => SECURITY_FAILURE_OPEN_REPAIR_WINDOW,
        }
    }
}

/// Single-shot Swift Pair prompt evaluation (protocol section 5).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwiftPairPromptEvaluation {
    Active,
    Suppressed,
    Consumed,
    Disabled,
    Expired,
}

impl SwiftPairPromptEvaluation {
    pub const fn as_u8(self) -> u8 {
        match self {
            Self::Active => SWIFT_PAIR_PROMPT_ACTIVE,
            Self::Suppressed => SWIFT_PAIR_PROMPT_SUPPRESSED,
            Self::Consumed => SWIFT_PAIR_PROMPT_CONSUMED,
            Self::Disabled => SWIFT_PAIR_PROMPT_DISABLED,
            Self::Expired => SWIFT_PAIR_PROMPT_EXPIRED,
        }
    }
}

/// Classify a disconnect status. `reason_code` is the host-stack status (HCI
/// error already composed); `locally_requested` comes from the caller's own
/// terminate bookkeeping.
pub const fn classify_disconnect(
    reason_code: i32,
    locally_requested: bool,
) -> DisconnectClassification {
    if locally_requested {
        return DisconnectClassification::LocalRequest;
    }
    if reason_code == HCI_HOST_DELIBERATE_DISCONNECT_STATUS as i32 {
        return DisconnectClassification::HostDeliberateUnpair;
    }
    DisconnectClassification::TransientLinkLoss
}

/// Advertising posture after a disconnect (no-chase rule).
pub const fn advertising_after_disconnect(
    classification: DisconnectClassification,
    bond_delete_active: bool,
) -> AdvertisingAfterDisconnect {
    if bond_delete_active {
        return AdvertisingAfterDisconnect::Suppress;
    }
    if matches!(
        classification,
        DisconnectClassification::HostDeliberateUnpair
    ) {
        return AdvertisingAfterDisconnect::UndirectedOnly;
    }
    AdvertisingAfterDisconnect::DirectedReconnect
}

/// Extra gate applied at advertising start; the caller also requires a bonded
/// peer address before asking.
pub const fn directed_reconnect_allowed(
    directed_pending: bool,
    low_power: bool,
    pairing_window_open: bool,
) -> bool {
    directed_pending && !low_power && !pairing_window_open
}

/// Bond-delete advertising suppression.
pub const fn advertising_allowed_during_bond_delete(
    bond_delete_active: bool,
    warmup_permitted: bool,
) -> bool {
    if bond_delete_active {
        warmup_permitted
    } else {
        false
    }
}

/// Window arithmetic.
pub const fn window_remaining_ms(opened_at_ms: i64, window_ms: i64, now_ms: i64) -> i64 {
    if opened_at_ms <= 0 || window_ms <= 0 {
        return 0;
    }
    let elapsed_ms = now_ms - opened_at_ms;
    if elapsed_ms < 0 {
        return window_ms;
    }
    if elapsed_ms >= window_ms {
        return 0;
    }
    window_ms - elapsed_ms
}

/// Bounded Swift Pair advertisement duration clamp.
pub const fn swift_pair_adv_duration_ms(remaining_ms: i64) -> i32 {
    if remaining_ms < SWIFT_PAIR_ADV_MIN_RESTART_MS as i64 {
        return SWIFT_PAIR_ADV_MIN_RESTART_MS as i32;
    }
    if remaining_ms > i32::MAX as i64 {
        return i32::MAX;
    }
    remaining_ms as i32
}

/// Single-shot Swift Pair prompt evaluation.
pub const fn evaluate_swift_pair_prompt(
    suppressed: bool,
    consumed: bool,
    prompt_window_ms: i64,
    prompt_remaining_ms: i64,
) -> SwiftPairPromptEvaluation {
    if suppressed {
        return SwiftPairPromptEvaluation::Suppressed;
    }
    if consumed {
        return SwiftPairPromptEvaluation::Consumed;
    }
    if prompt_window_ms <= 0 {
        return SwiftPairPromptEvaluation::Disabled;
    }
    if prompt_remaining_ms <= 0 {
        return SwiftPairPromptEvaluation::Expired;
    }
    SwiftPairPromptEvaluation::Active
}

/// Secure-connection window decision.
pub const fn window_close_on_secure(
    type_controlled: bool,
    window_open: bool,
) -> WindowCloseDecision {
    if type_controlled && window_open {
        return WindowCloseDecision::KeepOpen;
    }
    WindowCloseDecision::CloseNow
}

/// Type audio-ready window decision.
pub const fn window_close_on_type_audio_ready(
    window_open: bool,
    waiting_for_disconnect: bool,
    desc_valid: bool,
    encrypted: bool,
    bonded: bool,
) -> WindowCloseDecision {
    if window_open && !waiting_for_disconnect && desc_valid && (encrypted || bonded) {
        return WindowCloseDecision::CloseNow;
    }
    WindowCloseDecision::KeepOpen
}

/// Type-controlled recovery ownership test.
pub const fn type_controlled_recovery(
    type_controlled_request: bool,
    type_link_ready: bool,
    type_host_recent: bool,
    connected: bool,
) -> bool {
    type_controlled_request || type_link_ready || (type_host_recent && connected)
}

/// Identity action for a recovery pairing reset.
pub const fn identity_for_recovery(
    type_controlled_recovery: bool,
    connected: bool,
) -> IdentityAction {
    if type_controlled_recovery {
        return IdentityAction::KeepStable;
    }
    if connected {
        return IdentityAction::DeferRotateUntilDisconnect;
    }
    IdentityAction::RotateFresh
}

/// Deferred rotation ordering: rotation runs after the disconnect/bond
/// cleanup and before the recovery advertising start.
pub const fn rotate_before_advertising(identity_rotate_pending: bool) -> bool {
    identity_rotate_pending
}

/// In-place refresh of an already-open window.
pub const fn refresh_existing_pairing_window(
    window_open: bool,
    bonded_peer_count: i32,
    connected: bool,
    force_fresh_identity: bool,
) -> bool {
    window_open && bonded_peer_count == 0 && !connected && !force_fresh_identity
}

/// Security-failure repair decision.
pub const fn security_failure_action(pairing_window_open: bool) -> SecurityFailureAction {
    if pairing_window_open {
        return SecurityFailureAction::RetryWithinWindow;
    }
    SecurityFailureAction::OpenRepairWindow
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_host_delete_is_classified_and_never_chased() {
        let classification = classify_disconnect(531, false);
        assert_eq!(
            classification,
            DisconnectClassification::HostDeliberateUnpair
        );
        assert_eq!(classification.as_u8(), DISCONNECT_HOST_DELIBERATE_UNPAIR);
        assert_eq!(
            HCI_HOST_DELIBERATE_DISCONNECT_STATUS,
            HCI_NIMBLE_HCI_STATUS_BASE + HCI_REMOTE_USER_TERMINATED_REASON
        );
        assert_eq!(
            advertising_after_disconnect(classification, false),
            AdvertisingAfterDisconnect::UndirectedOnly
        );
    }

    #[test]
    fn transient_loss_keeps_directed_reconnect() {
        let classification = classify_disconnect(0x08, false);
        assert_eq!(classification, DisconnectClassification::TransientLinkLoss);
        assert_eq!(
            advertising_after_disconnect(classification, false),
            AdvertisingAfterDisconnect::DirectedReconnect
        );
        assert_eq!(classify_disconnect(530, false), classification);
        assert_eq!(classify_disconnect(532, false), classification);
    }

    #[test]
    fn local_request_overrides_reason_code() {
        assert_eq!(
            classify_disconnect(531, true),
            DisconnectClassification::LocalRequest
        );
    }

    #[test]
    fn bond_delete_suppresses_advertising() {
        assert_eq!(
            advertising_after_disconnect(DisconnectClassification::TransientLinkLoss, true),
            AdvertisingAfterDisconnect::Suppress
        );
        assert!(!advertising_allowed_during_bond_delete(true, false));
        assert!(advertising_allowed_during_bond_delete(true, true));
        assert!(!advertising_allowed_during_bond_delete(false, true));
    }

    #[test]
    fn directed_gate_requires_idle_undirected_context() {
        assert!(directed_reconnect_allowed(true, false, false));
        assert!(!directed_reconnect_allowed(false, false, false));
        assert!(!directed_reconnect_allowed(true, true, false));
        assert!(!directed_reconnect_allowed(true, false, true));
    }

    #[test]
    fn window_remaining_edges() {
        assert_eq!(window_remaining_ms(0, 120_000, 500), 0);
        assert_eq!(window_remaining_ms(100, 0, 500), 0);
        assert_eq!(window_remaining_ms(1000, 120_000, 500), 120_000);
        assert_eq!(window_remaining_ms(1000, 120_000, 120_999), 1);
        assert_eq!(window_remaining_ms(1000, 120_000, 121_000), 0);
        assert_eq!(window_remaining_ms(1000, 120_000, 61_000), 60_000);
    }

    #[test]
    fn swift_pair_duration_clamps() {
        assert_eq!(swift_pair_adv_duration_ms(0), 1000);
        assert_eq!(swift_pair_adv_duration_ms(999), 1000);
        assert_eq!(swift_pair_adv_duration_ms(45_000), 45_000);
        assert_eq!(swift_pair_adv_duration_ms(i32::MAX as i64 + 1), i32::MAX);
    }

    #[test]
    fn swift_pair_prompt_rows() {
        assert_eq!(
            evaluate_swift_pair_prompt(false, false, 45_000, 30_000),
            SwiftPairPromptEvaluation::Active
        );
        assert_eq!(
            evaluate_swift_pair_prompt(true, false, 45_000, 30_000),
            SwiftPairPromptEvaluation::Suppressed
        );
        assert_eq!(
            evaluate_swift_pair_prompt(false, true, 45_000, 30_000),
            SwiftPairPromptEvaluation::Consumed
        );
        assert_eq!(
            evaluate_swift_pair_prompt(false, false, 0, 30_000),
            SwiftPairPromptEvaluation::Disabled
        );
        assert_eq!(
            evaluate_swift_pair_prompt(false, false, 45_000, 0),
            SwiftPairPromptEvaluation::Expired
        );
    }

    #[test]
    fn secure_close_waits_for_type_audio_only_when_type_controlled() {
        assert_eq!(
            window_close_on_secure(true, true),
            WindowCloseDecision::KeepOpen
        );
        assert_eq!(
            window_close_on_secure(true, false),
            WindowCloseDecision::CloseNow
        );
        assert_eq!(
            window_close_on_secure(false, true),
            WindowCloseDecision::CloseNow
        );
    }

    #[test]
    fn type_ready_closes_only_on_secure_live_window() {
        assert_eq!(
            window_close_on_type_audio_ready(true, false, true, true, false),
            WindowCloseDecision::CloseNow
        );
        assert_eq!(
            window_close_on_type_audio_ready(true, false, true, false, true),
            WindowCloseDecision::CloseNow
        );
        assert_eq!(
            window_close_on_type_audio_ready(true, false, true, false, false),
            WindowCloseDecision::KeepOpen
        );
        assert_eq!(
            window_close_on_type_audio_ready(true, true, true, true, true),
            WindowCloseDecision::KeepOpen
        );
        assert_eq!(
            window_close_on_type_audio_ready(false, false, true, true, true),
            WindowCloseDecision::KeepOpen
        );
        assert_eq!(
            window_close_on_type_audio_ready(true, false, false, false, false),
            WindowCloseDecision::KeepOpen
        );
    }

    #[test]
    fn type_controlled_requires_request_link_or_live_recent_host() {
        assert!(type_controlled_recovery(true, false, false, false));
        assert!(type_controlled_recovery(false, true, false, false));
        assert!(type_controlled_recovery(false, false, true, true));
        assert!(!type_controlled_recovery(false, false, true, false));
        assert!(!type_controlled_recovery(false, false, false, true));
    }

    #[test]
    fn identity_policy_rows() {
        assert_eq!(
            identity_for_recovery(true, false),
            IdentityAction::KeepStable
        );
        assert_eq!(
            identity_for_recovery(false, true),
            IdentityAction::DeferRotateUntilDisconnect
        );
        assert_eq!(
            identity_for_recovery(false, false),
            IdentityAction::RotateFresh
        );
    }

    #[test]
    fn delete_then_rotate_then_advertise() {
        assert!(rotate_before_advertising(true));
        assert!(!rotate_before_advertising(false));
    }

    #[test]
    fn refresh_rows() {
        assert!(refresh_existing_pairing_window(true, 0, false, false));
        assert!(!refresh_existing_pairing_window(false, 0, false, false));
        assert!(!refresh_existing_pairing_window(true, 1, false, false));
        assert!(!refresh_existing_pairing_window(true, 0, true, false));
        assert!(!refresh_existing_pairing_window(true, 0, false, true));
    }

    #[test]
    fn security_failure_routing() {
        assert_eq!(
            security_failure_action(true),
            SecurityFailureAction::RetryWithinWindow
        );
        assert_eq!(
            security_failure_action(false),
            SecurityFailureAction::OpenRepairWindow
        );
    }
}
