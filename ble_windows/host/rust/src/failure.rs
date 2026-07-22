//! Platform-neutral BLE failure taxonomy shared by host applications.
//!
//! The classification is pure string matching over error text produced by the
//! Windows BLE helpers (or by product adapters). Product-specific keywords
//! (device names, wake-key labels, product-specific error phrasing) are
//! supplied by the caller through [`BleFailureHints`]; user-facing guidance
//! strings here stay product-neutral so adapters can override them per kind.

use serde::Serialize;

use crate::BleFailureKind;

/// Extra product-specific error substrings folded into classification.
///
/// Every needle is matched against the lowercased error text at the same
/// position in the decision chain as its generic counterparts.
#[derive(Debug, Clone, Copy, Default)]
pub struct BleFailureHints {
    /// Extra needles for [`BleFailureKind::BackgroundContention`].
    pub background_contention: &'static [&'static str],
    /// Extra needles for [`BleFailureKind::MissingPairing`].
    pub missing_pairing: &'static [&'static str],
    /// Extra needles for [`BleFailureKind::DeviceAsleep`].
    pub device_asleep: &'static [&'static str],
    /// Extra needles for [`BleFailureKind::DeviceMissing`].
    pub device_missing: &'static [&'static str],
}

impl BleFailureHints {
    pub const NONE: Self = Self {
        background_contention: &[],
        missing_pairing: &[],
        device_asleep: &[],
        device_missing: &[],
    };
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BleFailureClassification {
    pub kind: BleFailureKind,
    pub retryable: bool,
    pub automatic_recovery: bool,
    pub user_action: &'static str,
    pub evidence: String,
}

/// Classify a BLE failure with only the generic keyword set.
pub fn classify_ble_failure(error: &str) -> BleFailureClassification {
    classify_ble_failure_with_hints(error, &BleFailureHints::NONE)
}

/// Classify a BLE failure, folding product-specific keyword hints into the
/// generic decision chain.
pub fn classify_ble_failure_with_hints(
    error: &str,
    hints: &BleFailureHints,
) -> BleFailureClassification {
    let lower = error.to_ascii_lowercase();
    let kind = if lower.contains("only supported on windows") {
        BleFailureKind::UnsupportedPlatform
    } else if lower.contains("ota reboot")
        || lower.contains("after ota")
        || lower.contains("confirm")
        || lower.contains("reboot window")
    {
        BleFailureKind::OtaRebootWindow
    } else if lower.contains("background capture")
        || lower.contains("already active")
        || lower.contains("foreground probe skipped")
        || lower.contains("cancelled")
        || lower.contains("canceled")
        || hints
            .background_contention
            .iter()
            .any(|needle| lower.contains(needle))
    {
        BleFailureKind::BackgroundContention
    } else if lower.contains("firmware revision")
        || lower.contains("dis firmware")
        || lower.contains("firmware version")
    {
        BleFailureKind::MissingDisFirmwareRevision
    } else if lower.contains("cccd")
        || lower.contains("protocol_error")
        || lower.contains("protocol error")
        || lower.contains("notify write")
        || lower.contains("notify characteristic")
    {
        BleFailureKind::CccdProtocolError
    } else if lower.contains("bluetooth service")
        || lower.contains("radio")
        || lower.contains("adapter")
        || lower.contains("0x8007048f")
        || lower.contains("0x800710df")
        || lower.contains("service reset")
    {
        BleFailureKind::WindowsBluetoothServiceResetNeeded
    } else if lower.contains("access denied") || lower.contains("denied") {
        BleFailureKind::AccessDenied
    } else if ble_error_suggests_missing_pairing(&lower, hints) {
        BleFailureKind::MissingPairing
    } else if ble_error_suggests_device_asleep(&lower, hints) {
        BleFailureKind::DeviceAsleep
    } else if lower.contains("stale")
        || lower.contains("unknown gatt")
        || lower.contains("0x80070016")
        || (lower.contains("cached") && !lower.contains("uncached"))
        || lower.contains("gatt cache")
        || lower.contains("service changed")
    {
        BleFailureKind::StaleGattService
    } else if ble_error_suggests_low_power_idle_disconnect(&lower) {
        BleFailureKind::LowPowerIdleDisconnect
    } else if lower.contains("unreachable")
        || lower.contains("disconnected")
        || (lower.contains("transport_not_ready") && lower.contains("disconnect"))
        || lower.contains("timed out")
        || lower.contains("timeout")
    {
        BleFailureKind::PairedButDisconnected
    } else if lower.contains("not found")
        || lower.contains("no subscribable")
        || lower.contains("selector returned no")
        || lower.contains("returned no devices")
        || lower.contains("no devices")
        || lower.contains("no paired ble device")
        || hints
            .device_missing
            .iter()
            .any(|needle| lower.contains(needle))
    {
        BleFailureKind::DeviceMissing
    } else {
        BleFailureKind::Unknown
    };

    let (retryable, automatic_recovery, user_action) = failure_response(kind);

    BleFailureClassification {
        kind,
        retryable,
        automatic_recovery,
        user_action,
        evidence: error.chars().take(480).collect(),
    }
}

/// Retryability and product-neutral user guidance per failure kind.
pub fn failure_response(kind: BleFailureKind) -> (bool, bool, &'static str) {
    match kind {
        BleFailureKind::DeviceMissing => (
            true,
            false,
            "Wake the BLE device, confirm it is paired, then retry or re-pair.",
        ),
        BleFailureKind::DeviceAsleep => (
            true,
            false,
            "Wake the device, wait for it to return online, then retry.",
        ),
        BleFailureKind::MissingPairing => (
            true,
            false,
            "Pair the device in Windows Bluetooth settings, then retry.",
        ),
        BleFailureKind::LowPowerIdleDisconnect => (
            true,
            true,
            "The device entered a low-power offline state; retrying will reconnect.",
        ),
        BleFailureKind::PairedButDisconnected => (
            true,
            true,
            "Wait for automatic reconnect; wake the device if it stays disconnected.",
        ),
        BleFailureKind::StaleGattService => (
            true,
            true,
            "Retry after the host refreshes the GATT path; re-pair if stale services persist.",
        ),
        BleFailureKind::CccdProtocolError => (
            true,
            true,
            "Retry after the notify subscription is reopened; restart the host application if repeated.",
        ),
        BleFailureKind::MissingDisFirmwareRevision => (
            false,
            false,
            "Collect diagnostics and update firmware readiness/DIS exposure before release.",
        ),
        BleFailureKind::BackgroundContention => (
            true,
            true,
            "Pause the competing BLE operation and retry through the shared BLE path.",
        ),
        BleFailureKind::OtaRebootWindow => (
            true,
            true,
            "Wait for the OTA reboot window to finish, then refresh device status.",
        ),
        BleFailureKind::WindowsBluetoothServiceResetNeeded => (
            true,
            false,
            "Toggle Windows Bluetooth or restart the Bluetooth Support Service, then retry.",
        ),
        BleFailureKind::AccessDenied => (
            false,
            false,
            "Allow Bluetooth/device access in Windows settings or re-pair the device.",
        ),
        BleFailureKind::UnsupportedPlatform => (
            false,
            false,
            "Use the supported Windows BLE path for this diagnostic.",
        ),
        BleFailureKind::Unknown => (
            true,
            false,
            "Export diagnostics and retry after restarting the host application.",
        ),
    }
}

fn ble_error_suggests_missing_pairing(lower: &str, hints: &BleFailureHints) -> bool {
    lower.contains("no paired ble device")
        || lower.contains("not paired")
        || lower.contains("missing pairing")
        || lower.contains("pairing missing")
        || hints
            .missing_pairing
            .iter()
            .any(|needle| lower.contains(needle))
}

fn ble_error_suggests_device_asleep(lower: &str, hints: &BleFailureHints) -> bool {
    lower.contains("deep sleep")
        || lower.contains("asleep")
        || lower.contains("sleeping")
        || lower.contains("wake key")
        || hints
            .device_asleep
            .iter()
            .any(|needle| lower.contains(needle))
}

fn ble_error_suggests_low_power_idle_disconnect(lower: &str) -> bool {
    let reason_546 = lower.contains("reason=546")
        || lower.contains("reason: 546")
        || lower.contains("reason 546")
        || lower.contains("reason=0x222")
        || lower.contains("reason: 0x222");
    let idle_label = lower.contains("low-power idle")
        || lower.contains("low power idle")
        || lower.contains("idle disconnect")
        || lower.contains("idle-disconnect")
        || lower.contains("intentional idle");
    let transport_not_ready =
        lower.contains("transport_not_ready") || lower.contains("transport not ready");

    reason_546 || idle_label || (transport_not_ready && lower.contains("low power"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kind_of(error: &str) -> BleFailureKind {
        classify_ble_failure(error).kind
    }

    #[test]
    fn classifies_each_generic_family() {
        assert_eq!(
            kind_of("BLE capture is only supported on windows"),
            BleFailureKind::UnsupportedPlatform
        );
        assert_eq!(
            kind_of("device is in the OTA reboot window"),
            BleFailureKind::OtaRebootWindow
        );
        assert_eq!(
            kind_of("a background capture is already active"),
            BleFailureKind::BackgroundContention
        );
        assert_eq!(
            kind_of("BLE notify read cancelled"),
            BleFailureKind::BackgroundContention
        );
        assert_eq!(
            kind_of("missing DIS firmware revision"),
            BleFailureKind::MissingDisFirmwareRevision
        );
        assert_eq!(
            kind_of("CCCD write returned protocol_error=146"),
            BleFailureKind::CccdProtocolError
        );
        assert_eq!(
            kind_of("bluetooth service reset needed: 0x8007048F"),
            BleFailureKind::WindowsBluetoothServiceResetNeeded
        );
        assert_eq!(kind_of("access denied"), BleFailureKind::AccessDenied);
        assert_eq!(
            kind_of("no paired BLE device matched"),
            BleFailureKind::MissingPairing
        );
        assert_eq!(
            kind_of("device is in deep sleep; press the wake key"),
            BleFailureKind::DeviceAsleep
        );
        assert_eq!(
            kind_of("stale GATT service entry (service changed)"),
            BleFailureKind::StaleGattService
        );
        assert_eq!(
            kind_of("disconnect reason=546 received"),
            BleFailureKind::LowPowerIdleDisconnect
        );
        assert_eq!(
            kind_of("transport_not_ready while low power"),
            BleFailureKind::LowPowerIdleDisconnect
        );
        assert_eq!(
            kind_of("BLE device open timed out after 15000 ms"),
            BleFailureKind::PairedButDisconnected
        );
        assert_eq!(
            kind_of("selector returned no devices"),
            BleFailureKind::DeviceMissing
        );
        assert_eq!(kind_of("something else entirely"), BleFailureKind::Unknown);
    }

    #[test]
    fn generic_classifier_ignores_product_keywords_without_hints() {
        assert_eq!(
            kind_of("no paired acme-device present"),
            BleFailureKind::Unknown
        );
        assert_eq!(kind_of("press KEY9 to wake"), BleFailureKind::Unknown);
    }

    #[test]
    fn hints_extend_the_decision_chain_in_position() {
        let hints = BleFailureHints {
            background_contention: &["background acme"],
            missing_pairing: &["no paired acme-device", "pair the acme"],
            device_asleep: &["press key9", "key9"],
            device_missing: &["no writable acme ble ota"],
        };
        let classify = |error: &str| classify_ble_failure_with_hints(error, &hints).kind;
        assert_eq!(
            classify("BLE notify cancelled by background acme recovery"),
            BleFailureKind::BackgroundContention
        );
        assert_eq!(
            classify("no paired acme-device present"),
            BleFailureKind::MissingPairing
        );
        assert_eq!(classify("press KEY9 to wake"), BleFailureKind::DeviceAsleep);
        assert_eq!(
            classify("No writable acme BLE OTA characteristic found on device"),
            BleFailureKind::DeviceMissing
        );
        // Hint precedence matches the generic chain: contention is checked
        // before pairing, and timeout stays paired-but-disconnected.
        assert_eq!(
            classify("device open timed out; pair the acme"),
            BleFailureKind::MissingPairing
        );
    }

    #[test]
    fn response_mapping_matches_kind_contract() {
        let (retryable, automatic_recovery, user_action) =
            failure_response(BleFailureKind::LowPowerIdleDisconnect);
        assert!(retryable && automatic_recovery && !user_action.is_empty());
        let (retryable, automatic_recovery, _) = failure_response(BleFailureKind::AccessDenied);
        assert!(!retryable && !automatic_recovery);
        let (retryable, _, _) = failure_response(BleFailureKind::Unknown);
        assert!(retryable);
        for code in 0..=13u8 {
            let kind = match code {
                0 => BleFailureKind::DeviceMissing,
                1 => BleFailureKind::DeviceAsleep,
                2 => BleFailureKind::MissingPairing,
                3 => BleFailureKind::LowPowerIdleDisconnect,
                4 => BleFailureKind::PairedButDisconnected,
                5 => BleFailureKind::StaleGattService,
                6 => BleFailureKind::CccdProtocolError,
                7 => BleFailureKind::MissingDisFirmwareRevision,
                8 => BleFailureKind::BackgroundContention,
                9 => BleFailureKind::OtaRebootWindow,
                10 => BleFailureKind::WindowsBluetoothServiceResetNeeded,
                11 => BleFailureKind::AccessDenied,
                12 => BleFailureKind::UnsupportedPlatform,
                _ => BleFailureKind::Unknown,
            };
            let (_, _, user_action) = failure_response(kind);
            assert!(!user_action.is_empty());
        }
    }

    #[test]
    fn evidence_is_truncated_to_480_chars() {
        let long_error = "x".repeat(1000);
        let classification = classify_ble_failure(&long_error);
        assert_eq!(classification.evidence.chars().count(), 480);
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn failure_kind_serializes_camel_case() {
        let value = serde_json::to_string(&BleFailureKind::CccdProtocolError).unwrap();
        assert_eq!(value, "\"cccdProtocolError\"");
    }
}
