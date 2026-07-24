// Generated from device_health/protocol/device_health_v1.json. Do not edit.
use serde::{Deserialize, Serialize};

pub const CONTRACT_NAME: &str = "denzic_device_health_v1";
pub const CONTRACT_VERSION: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum ResetReason {
    Unknown = 0,
    PowerOn = 1,
    External = 2,
    Software = 3,
    Panic = 4,
    Watchdog = 5,
    DeepSleep = 6,
    Brownout = 7,
    Usb = 8,
    Jtag = 9,
    PowerGlitch = 10,
    CpuLockup = 11,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum CheckState {
    Passed = 0,
    Failed = 1,
    NotRequired = 2,
    Recovered = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeAlert {
    None = 0,
    HeapPressure = 1,
    LinkChurn = 2,
}

pub const SAFE_MODE_THRESHOLD: u32 = 3;
