// Generated from device_control/protocol/device_control_v1.json. Do not edit.
use serde::{Deserialize, Serialize};

pub const CONTRACT_NAME: &str = "denzic_device_control_v1";
pub const CONTRACT_VERSION: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum Transport {
    Unknown = 0,
    Ble = 1,
    Usb = 2,
    Wifi = 3,
    Serial = 4,
    Other = 255,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum LifecycleState {
    Unknown = 0,
    Disconnected = 1,
    Discovering = 2,
    Connecting = 3,
    Securing = 4,
    Ready = 5,
    Recovering = 6,
    OwnedElsewhere = 7,
    Failed = 8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum OwnershipState {
    Unknown = 0,
    Unpaired = 1,
    Local = 2,
    ManualUnpaired = 3,
    External = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum OperationKind {
    Unknown = 0,
    Discover = 1,
    Connect = 2,
    Secure = 3,
    ReadCapabilities = 4,
    ReadSetting = 5,
    WriteSetting = 6,
    InvokeCommand = 7,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum OperationResult {
    Accepted = 1,
    Succeeded = 2,
    Rejected = 3,
    Cancelled = 4,
    TimedOut = 5,
    TransportLost = 6,
    Failed = 7,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum ErrorCategory {
    None = 0,
    Device = 1,
    Transport = 2,
    Security = 3,
    Host = 4,
    Protocol = 5,
    Ownership = 6,
    Unsupported = 7,
    Timeout = 8,
    Resource = 9,
}
