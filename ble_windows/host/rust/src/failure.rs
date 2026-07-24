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

/// Session-failure classification: is the error a link loss (fast retry)?
///
/// Pure keyword matching over transport error text (WinRT status text, host
/// BLE helper messages, firmware disconnect reasons). What a link loss means
/// for retry cadence or user guidance stays in the calling product adapter.
pub fn is_ble_link_loss_error(error: &str) -> bool {
    let lower = error.to_ascii_lowercase();
    lower.contains("connection status changed")
        || lower.contains("gatt session status changed")
        || (lower.contains("notification wait failed") && lower.contains("disconnected"))
        || lower.contains("transport_not_ready")
        || lower.contains("transport not ready")
        || lower.contains("reason=546")
        || lower.contains("reason: 546")
        || lower.contains("reason 546")
        || lower.contains("low-power idle")
        || lower.contains("low power idle")
        || lower.contains("idle disconnect")
}

/// Session-failure classification: is the error a transient reopen failure?
///
/// Matches the case-sensitive WinRT/Helper status formatting on purpose: the
/// needles carry the exact debug rendering the Windows BLE helpers emit, so
/// matching is done against the original text, not a lowercased copy.
pub fn is_ble_transient_reopen_error(error: &str) -> bool {
    error.contains("GattCommunicationStatus(1)")
        || error.contains("GattCommunicationStatus(3)")
        || error.contains("HRESULT(0x800706BA)")
        || error.contains("BLE characteristic discovery returned status")
        || error.contains("BLE service open wait failed")
}

/// Session-failure classification: should retries back off while the peer is
/// offline (asleep, disconnected, or carrying a stale GATT/cache view)?
pub fn is_ble_offline_backoff_error(error: &str) -> bool {
    let lower = error.to_ascii_lowercase();
    lower.contains("gatt session did not become active")
        || lower.contains("paired device disconnected")
        || lower.contains("stale gatt/cache")
        || lower.contains("device is asleep")
        || lower.contains("device asleep")
        || lower.contains("wake key")
        || lower.contains("not found from service selector")
}

/// Session-failure classification: is the error a known-noisy CCCD failure?
///
/// A CCCD failure only counts as noise when the failure taxonomy also
/// classifies it as a CCCD protocol error; `hints` must be the same product
/// hints the caller uses for its own classification so both layers agree.
pub fn is_ble_noisy_cccd_failure(error: &str, hints: &BleFailureHints) -> bool {
    if classify_ble_failure_with_hints(error, hints).kind != BleFailureKind::CccdProtocolError {
        return false;
    }
    let lower = error.to_ascii_lowercase();
    lower.contains("hresult(0x800704c7)")
        || lower.contains("cccd write timed out")
        || lower.contains("gattcommunicationstatus(1)")
        || lower.contains("protocol_error=3")
        || lower.contains("protocol error=3")
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

    #[test]
    fn link_loss_covers_status_change_and_idle_disconnect_families() {
        assert!(is_ble_link_loss_error(
            "BLE device connection status changed to Disconnected; transport_not_ready"
        ));
        assert!(is_ble_link_loss_error("gatt session status changed"));
        assert!(is_ble_link_loss_error(
            "BLE embedded audio notification wait failed: disconnected"
        ));
        assert!(is_ble_link_loss_error(
            "Windows BLE disconnected; reason=546; low-power idle"
        ));
        assert!(is_ble_link_loss_error("disconnect reason: 546"));
        assert!(is_ble_link_loss_error("idle disconnect"));
        assert!(!is_ble_link_loss_error(
            "BLE embedded audio notification wait failed: channel closed unexpectedly"
        ));
        assert!(!is_ble_link_loss_error("element not found"));
    }

    #[test]
    fn transient_reopen_matches_case_sensitive_status_rendering() {
        assert!(is_ble_transient_reopen_error(
            "open failed: GattCommunicationStatus(1)"
        ));
        assert!(is_ble_transient_reopen_error("GattCommunicationStatus(3)"));
        assert!(is_ble_transient_reopen_error("HRESULT(0x800706BA)"));
        assert!(is_ble_transient_reopen_error(
            "BLE characteristic discovery returned status 3"
        ));
        assert!(is_ble_transient_reopen_error(
            "BLE service open wait failed"
        ));
        // The needles are the exact WinRT debug rendering; lowercase text is
        // not a transient reopen signature.
        assert!(!is_ble_transient_reopen_error("gattcommunicationstatus(1)"));
        assert!(!is_ble_transient_reopen_error("hresult(0x800706ba)"));
    }

    #[test]
    fn offline_backoff_covers_asleep_disconnected_and_stale_cache() {
        assert!(is_ble_offline_backoff_error(
            "BLE GATT session did not become active after 8000 ms"
        ));
        assert!(is_ble_offline_backoff_error("paired device disconnected"));
        assert!(is_ble_offline_backoff_error("stale GATT/cache view"));
        assert!(is_ble_offline_backoff_error("device is asleep"));
        assert!(is_ble_offline_backoff_error("press the wake key"));
        assert!(is_ble_offline_backoff_error(
            "service not found from service selector"
        ));
        assert!(!is_ble_offline_backoff_error("GattCommunicationStatus(1)"));
    }

    #[test]
    fn noisy_cccd_requires_cccd_kind_plus_noise_signature() {
        let hints = BleFailureHints::NONE;
        assert!(is_ble_noisy_cccd_failure(
            "CCCD write failed: HRESULT(0x800704C7)",
            &hints
        ));
        assert!(is_ble_noisy_cccd_failure("cccd write timed out", &hints));
        assert!(is_ble_noisy_cccd_failure(
            "notify write failed: GattCommunicationStatus(1)",
            &hints
        ));
        assert!(is_ble_noisy_cccd_failure("protocol_error=3", &hints));
        assert!(is_ble_noisy_cccd_failure("protocol error=3", &hints));
        // Same noise signature without a CCCD classification is not noise.
        assert!(!is_ble_noisy_cccd_failure(
            "service open failed: HRESULT(0x800704C7)",
            &hints
        ));
        // Product hints participate in the kind check exactly like the
        // caller's own classification does.
        let listener_hints = BleFailureHints {
            background_contention: &["background listener"],
            ..BleFailureHints::NONE
        };
        assert!(!is_ble_noisy_cccd_failure(
            "CCCD write failed during background listener capture: HRESULT(0x800704C7)",
            &listener_hints
        ));
    }

    #[cfg(target_os = "windows")]
    #[test]
    fn failure_kind_serializes_camel_case() {
        let value = serde_json::to_string(&BleFailureKind::CccdProtocolError).unwrap();
        assert_eq!(value, "\"cccdProtocolError\"");
    }
}
