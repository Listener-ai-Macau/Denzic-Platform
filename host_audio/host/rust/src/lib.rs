//! Host audio v1 core: product-independent host microphone capture, WAV
//! container handling, ASR provider contracts, Companion-compatible
//! IMA-ADPCM, and **product-default Opus-16** host encode/decode path.
//!
//! Wire constants (16 kHz / mono / 16-bit little-endian PCM, the 44-byte WAV
//! header layout, and the OpenAI-compatible ASR defaults) are generated from
//! `host_audio/protocol/host_audio_v1.json` into `generated.rs`. Products
//! supply session management, file placement, resampling policy, credential
//! plumbing, and UI. Volcengine SAUC streaming remains a platform follow-up
//! (PLAT-REQ-003); batch OpenAI-compatible ASR ships here.

mod generated;

pub mod asr;
pub mod asr_openai;
pub mod http_policy;
pub mod capture;
pub mod ima_adpcm;
pub mod opus_codec;
#[cfg(feature = "native-opus")]
pub mod opus_decoder;
pub mod wav;

pub use generated::*;
pub use ima_adpcm::{
    compress_pcm16k_half_ima, decompress_half_ima_to_pcm16k, downsample_half, encode_frame,
    decode_frame, upsample_x2_hold, ImaAdpcmState, COMPANION_RECORDING_REGION_BYTES,
    HALF_IMA_BYTES_PER_SEC, RAW_PCM16K_BYTES_PER_SEC, max_seconds_for_store_rate,
};
pub use opus_codec::{
    decode_opus_to_wav, encode_wav_to_opus, fits_recording_region, opus16_max_seconds,
    round_trip_opus16, OPUS_16KBPS_BYTES_PER_SEC, OPUS_PRODUCT_BITRATE_BPS,
};
