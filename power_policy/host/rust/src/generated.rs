// Generated from power_policy/protocol/power_policy_v1.json. Do not edit.
use serde::{Deserialize, Serialize};

pub const CONTRACT_NAME: &str = "denzic_power_policy_v1";
pub const CONTRACT_VERSION: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum Blocker {
    Recording = 0,
    AudioTransport = 1,
    Diagnostic = 2,
    Pairing = 3,
    Reconnect = 4,
    Storage = 5,
    Command = 6,
    ExternalPower = 7,
    Ota = 8,
    Touch = 9,
    Menu = 10,
    Display = 11,
    Boot = 12,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Active = 0,
    ConnectedIdle = 1,
    DisconnectedIdle = 2,
    ShutdownRequested = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum ShutdownReason {
    None = 0,
    LongIdle = 1,
    Manual = 2,
    LowBattery = 3,
}

pub const MAX_BLOCKER_INDEX: u32 = 31;
