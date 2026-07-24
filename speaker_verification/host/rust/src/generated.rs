// Generated from speaker_verification/protocol/speaker_verification_v1.json. Do not edit.
use serde::{Deserialize, Serialize};

pub const CONTRACT_NAME: &str = "denzic_speaker_verification_v1";
pub const CONTRACT_VERSION: u8 = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum CandidateOrigin {
    Manual = 0,
    Automatic = 1,
    Enrollment = 2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum State {
    Idle = 0,
    Buffering = 1,
    Released = 2,
    Discarded = 3,
    Error = 4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Pending = 0,
    Match = 1,
    NonMatch = 2,
    Unavailable = 3,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    None = 0,
    Buffer = 1,
    Release = 2,
    Discard = 3,
    Bypass = 4,
}

pub const DEFAULT_SCORE_MILLI: u32 = 500;
pub const DEFAULT_MIN_CANDIDATE_MS: u32 = 1000;
pub const DEFAULT_MAX_CANDIDATE_MS: u32 = 10000;
