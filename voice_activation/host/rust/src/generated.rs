// Generated from voice_activation/protocol/voice_activation_v1.json. Do not edit.
use serde::{Deserialize, Serialize};

pub const CONTRACT_NAME: &str = "denzic_voice_activation_v1";
pub const CONTRACT_VERSION: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Disabled = 0,
    Monitoring = 1,
    SpeechConfirming = 2,
    Recording = 3,
    Tail = 4,
    Cooldown = 5,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    None = 0,
    Start = 1,
    Stop = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum StopReason {
    None = 0,
    Silence = 1,
    MaxDuration = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum PhraseSignal {
    None = 0,
    KeywordModel = 1,
    LocalTranscript = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum GateDecision {
    Pending = 0,
    Accept = 1,
    Reject = 2,
}

pub const DEFAULT_SPEECH_CONFIRM_MS: u32 = 300;
pub const DEFAULT_PRE_ROLL_MS: u32 = 600;
pub const DEFAULT_SILENCE_STOP_MS: u32 = 1000;
pub const DEFAULT_TAIL_MS: u32 = 250;
pub const DEFAULT_MIN_SESSION_MS: u32 = 500;
pub const DEFAULT_MAX_SESSION_MS: u32 = 60000;
pub const DEFAULT_COOLDOWN_MS: u32 = 1500;
pub const DEFAULT_LOCAL_CONFIRMATION_START_MS: u32 = 1800;
