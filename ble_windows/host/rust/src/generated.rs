// Generated from ble_windows/protocol/ble_windows_v1.json. Do not edit.
use serde::{Deserialize, Serialize};

pub const CONTRACT_NAME: &str = "denzic_ble_windows_v1";
pub const CONTRACT_VERSION: u8 = 1;

pub const DEFAULT_DISCOVERY_TIMEOUT_MS: u64 = 15000;
pub const OPTIONAL_READ_TIMEOUT_MS: u64 = 2000;
pub const CCCD_ENABLE_TIMEOUT_MS: u64 = 8000;
pub const ASYNC_POLL_INTERVAL_MS: u64 = 25;
pub const CREATE_NO_WINDOW_FLAG: u32 = 134217728;
pub const CCCD_ENABLE_RETRY_DELAYS_MS: [u64; 3] = [250, 750, 1500];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[serde(rename_all = "camelCase")]
pub enum BleFailureKind {
    DeviceMissing = 0,
    DeviceAsleep = 1,
    MissingPairing = 2,
    LowPowerIdleDisconnect = 3,
    PairedButDisconnected = 4,
    StaleGattService = 5,
    CccdProtocolError = 6,
    MissingDisFirmwareRevision = 7,
    BackgroundContention = 8,
    OtaRebootWindow = 9,
    WindowsBluetoothServiceResetNeeded = 10,
    AccessDenied = 11,
    UnsupportedPlatform = 12,
    Unknown = 13,
}
