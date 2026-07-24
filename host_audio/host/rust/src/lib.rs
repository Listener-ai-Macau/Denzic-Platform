//! Host audio v1 core: product-independent host microphone capture, WAV
//! container handling, and ASR provider contracts.
//!
//! Wire constants (16 kHz / mono / 16-bit little-endian PCM, the 44-byte WAV
//! header layout, and the OpenAI-compatible ASR defaults) are generated from
//! `host_audio/protocol/host_audio_v1.json` into `generated.rs`. Products
//! supply session management, file placement, resampling policy, and any
//! ASR providers beyond the OpenAI-compatible batch client shipped here.

mod generated;

pub mod asr;
pub mod asr_openai;
pub mod capture;
pub mod wav;

pub use generated::*;
