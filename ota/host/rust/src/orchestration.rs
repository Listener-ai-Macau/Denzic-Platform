//! Host-side mirror of the embedded OTA orchestration decision core
//! (`ota/embedded/c/src/denzic_ota_orchestration_v1.c`).
//!
//! The device-side C core owns the product-independent OTA orchestration
//! decisions (blocker gating, battery threshold, image-size admission,
//! inactivity timeout, pending-verify confirm/rollback). These functions
//! mirror that logic for host tooling and tests; keep the two in sync.

/// Default battery gate: OTA is blocked below this charge level.
pub const ORCHESTRATION_MIN_BATTERY_PERCENT: u8 = 20;
/// Default stale-session timeout: three minutes without transfer activity.
pub const ORCHESTRATION_INACTIVITY_TIMEOUT_MS: u32 = 3 * 60 * 1000;

pub const ROLLBACK_REASON_POST_FAILED: u32 = 0x01;
pub const ROLLBACK_REASON_BLE_NOT_READY: u32 = 0x02;
pub const ROLLBACK_REASON_KEYBOARD_NOT_READY: u32 = 0x04;

/// OTA admission blocker, in priority order. Discriminants match
/// `denzic_ota_orchestration_v1_blocker_t`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum EmbeddedOtaBlocker {
    None = 0,
    InProgress = 1,
    PendingVerify = 2,
    RecordingActive = 3,
    BleAudioActive = 4,
    DiagExportActive = 5,
    LowBattery = 6,
    NoPartition = 7,
}

/// Device facts the product adapter samples and injects; the decision core
/// never reads sensors or activity flags itself.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct EmbeddedOtaBlockerInputs {
    pub session_active: bool,
    pub running_pending_verify: bool,
    pub recording_active: bool,
    pub ble_audio_active: bool,
    pub diag_export_active: bool,
    pub battery_valid: bool,
    pub battery_percent: u8,
    pub has_update_partition: bool,
}

pub fn evaluate_embedded_blocker(
    inputs: &EmbeddedOtaBlockerInputs,
    min_battery_percent: u8,
) -> EmbeddedOtaBlocker {
    if inputs.session_active {
        EmbeddedOtaBlocker::InProgress
    } else if inputs.running_pending_verify {
        EmbeddedOtaBlocker::PendingVerify
    } else if inputs.recording_active {
        EmbeddedOtaBlocker::RecordingActive
    } else if inputs.ble_audio_active {
        EmbeddedOtaBlocker::BleAudioActive
    } else if inputs.diag_export_active {
        EmbeddedOtaBlocker::DiagExportActive
    } else if inputs.battery_valid && inputs.battery_percent < min_battery_percent {
        EmbeddedOtaBlocker::LowBattery
    } else if !inputs.has_update_partition {
        EmbeddedOtaBlocker::NoPartition
    } else {
        EmbeddedOtaBlocker::None
    }
}

/// Mirrors the begin admission rule: a known image size must fit the target
/// partition; zero or the product's unknown-size sentinel always pass.
pub fn embedded_image_size_accepted(
    image_size: u64,
    unknown_size_sentinel: u64,
    partition_size: u64,
) -> bool {
    image_size == 0 || image_size == unknown_size_sentinel || image_size <= partition_size
}

/// Finish requires all expected bytes; an unknown expected size always passes.
pub fn embedded_finish_size_matches(bytes_written: u64, expected_size: u64) -> bool {
    expected_size == 0 || bytes_written == expected_size
}

pub fn embedded_inactivity_deadline_us(now_us: i64, timeout_ms: u32) -> i64 {
    now_us + i64::from(timeout_ms) * 1000
}

pub fn embedded_inactivity_expired(deadline_us: i64, now_us: i64) -> bool {
    now_us >= deadline_us
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum EmbeddedPendingAction {
    None = 0,
    Confirm = 1,
    Rollback = 2,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EmbeddedPendingDecision {
    pub action: EmbeddedPendingAction,
    pub rollback_reason_mask: u32,
}

/// Pending-verify decision: only a running pending-verify image is confirmed
/// or rolled back, and rollback carries a reason bit per failed self-check.
pub fn decide_embedded_pending_verify(
    running_pending_verify: bool,
    post_ok: bool,
    ble_ready: bool,
    keyboard_ready: bool,
) -> EmbeddedPendingDecision {
    if !running_pending_verify {
        return EmbeddedPendingDecision {
            action: EmbeddedPendingAction::None,
            rollback_reason_mask: 0,
        };
    }
    if post_ok && ble_ready && keyboard_ready {
        return EmbeddedPendingDecision {
            action: EmbeddedPendingAction::Confirm,
            rollback_reason_mask: 0,
        };
    }
    let mut rollback_reason_mask = 0;
    if !post_ok {
        rollback_reason_mask |= ROLLBACK_REASON_POST_FAILED;
    }
    if !ble_ready {
        rollback_reason_mask |= ROLLBACK_REASON_BLE_NOT_READY;
    }
    if !keyboard_ready {
        rollback_reason_mask |= ROLLBACK_REASON_KEYBOARD_NOT_READY;
    }
    EmbeddedPendingDecision {
        action: EmbeddedPendingAction::Rollback,
        rollback_reason_mask,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn idle_inputs() -> EmbeddedOtaBlockerInputs {
        EmbeddedOtaBlockerInputs {
            battery_valid: true,
            battery_percent: 100,
            has_update_partition: true,
            ..EmbeddedOtaBlockerInputs::default()
        }
    }

    #[test]
    fn blocker_priority_matches_embedded_core() {
        let mut inputs = idle_inputs();
        assert_eq!(
            evaluate_embedded_blocker(&inputs, ORCHESTRATION_MIN_BATTERY_PERCENT),
            EmbeddedOtaBlocker::None
        );

        inputs.battery_percent = 19;
        assert_eq!(
            evaluate_embedded_blocker(&inputs, ORCHESTRATION_MIN_BATTERY_PERCENT),
            EmbeddedOtaBlocker::LowBattery
        );

        inputs.battery_valid = false;
        assert_eq!(
            evaluate_embedded_blocker(&inputs, ORCHESTRATION_MIN_BATTERY_PERCENT),
            EmbeddedOtaBlocker::None
        );

        inputs.has_update_partition = false;
        assert_eq!(
            evaluate_embedded_blocker(&inputs, ORCHESTRATION_MIN_BATTERY_PERCENT),
            EmbeddedOtaBlocker::NoPartition
        );

        inputs.battery_valid = true;
        assert_eq!(
            evaluate_embedded_blocker(&inputs, ORCHESTRATION_MIN_BATTERY_PERCENT),
            EmbeddedOtaBlocker::LowBattery,
            "low battery outranks a missing partition"
        );

        inputs.diag_export_active = true;
        assert_eq!(
            evaluate_embedded_blocker(&inputs, ORCHESTRATION_MIN_BATTERY_PERCENT),
            EmbeddedOtaBlocker::DiagExportActive
        );
        inputs.ble_audio_active = true;
        assert_eq!(
            evaluate_embedded_blocker(&inputs, ORCHESTRATION_MIN_BATTERY_PERCENT),
            EmbeddedOtaBlocker::BleAudioActive
        );
        inputs.recording_active = true;
        assert_eq!(
            evaluate_embedded_blocker(&inputs, ORCHESTRATION_MIN_BATTERY_PERCENT),
            EmbeddedOtaBlocker::RecordingActive
        );
        inputs.running_pending_verify = true;
        assert_eq!(
            evaluate_embedded_blocker(&inputs, ORCHESTRATION_MIN_BATTERY_PERCENT),
            EmbeddedOtaBlocker::PendingVerify
        );
        inputs.session_active = true;
        assert_eq!(
            evaluate_embedded_blocker(&inputs, ORCHESTRATION_MIN_BATTERY_PERCENT),
            EmbeddedOtaBlocker::InProgress
        );
    }

    #[test]
    fn image_size_admission_matches_embedded_core() {
        const UNKNOWN: u64 = u32::MAX as u64;
        assert!(embedded_image_size_accepted(0, UNKNOWN, 1024));
        assert!(embedded_image_size_accepted(UNKNOWN, UNKNOWN, 1024));
        assert!(embedded_image_size_accepted(1024, UNKNOWN, 1024));
        assert!(!embedded_image_size_accepted(1025, UNKNOWN, 1024));
    }

    #[test]
    fn finish_size_rule_matches_embedded_core() {
        assert!(embedded_finish_size_matches(0, 0));
        assert!(embedded_finish_size_matches(64, 64));
        assert!(!embedded_finish_size_matches(63, 64));
    }

    #[test]
    fn inactivity_rules_match_embedded_core() {
        let deadline =
            embedded_inactivity_deadline_us(1_000_000, ORCHESTRATION_INACTIVITY_TIMEOUT_MS);
        assert_eq!(deadline, 1_000_000 + 180_000_000);
        assert!(!embedded_inactivity_expired(deadline, deadline - 1));
        assert!(embedded_inactivity_expired(deadline, deadline));
    }

    #[test]
    fn pending_verify_decision_matches_embedded_core() {
        assert_eq!(
            decide_embedded_pending_verify(false, false, false, false).action,
            EmbeddedPendingAction::None
        );
        let confirm = decide_embedded_pending_verify(true, true, true, true);
        assert_eq!(confirm.action, EmbeddedPendingAction::Confirm);
        assert_eq!(confirm.rollback_reason_mask, 0);

        let rollback = decide_embedded_pending_verify(true, false, true, false);
        assert_eq!(rollback.action, EmbeddedPendingAction::Rollback);
        assert_eq!(
            rollback.rollback_reason_mask,
            ROLLBACK_REASON_POST_FAILED | ROLLBACK_REASON_KEYBOARD_NOT_READY
        );
    }
}
