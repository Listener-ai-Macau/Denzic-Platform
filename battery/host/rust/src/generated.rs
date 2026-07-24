// Generated from battery/protocol/battery_v1.json. Do not edit.
use serde::{Deserialize, Serialize};

pub const CONTRACT_NAME: &str = "denzic_battery_v1";
pub const CONTRACT_VERSION: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum ChargeState {
    Unknown = 0,
    Discharging = 1,
    Charging = 2,
    Full = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum NotifyReason {
    None = 0,
    Initial = 1,
    LevelDelta = 2,
    Periodic = 3,
    Forced = 4,
}

pub const BATTERY_SERVICE_UUID16: u32 = 6159;
pub const BATTERY_LEVEL_UUID16: u32 = 10777;
pub const INVALID_LEVEL: u32 = 255;
pub const DEFAULT_EMPTY_MV: u32 = 2850;
pub const DEFAULT_FULL_MV: u32 = 4150;
pub const NOTIFY_THRESHOLD_PERCENT: u32 = 1;
pub const FORCE_REFRESH_INTERVAL_MS: u32 = 60000;
pub const CHARGE_FULL_MIN_MV: u32 = 4050;
pub const CHARGE_FULL_MIN_PERCENT: u32 = 88;
pub const CHARGE_FULL_DEBOUNCE_MS: u32 = 10000;
