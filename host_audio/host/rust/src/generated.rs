// Generated from host_audio/protocol/host_audio_v1.json. Do not edit.
pub const CONTRACT_NAME: &str = "denzic_host_audio_v1";
pub const CONTRACT_VERSION: u8 = 1;
pub const PCM_SAMPLE_RATE_HZ: u32 = 16000;
pub const PCM_CHANNELS: u16 = 1;
pub const PCM_SAMPLE_WIDTH_BITS: u16 = 16;
pub const PCM_BLOCK_ALIGN: u16 = PCM_CHANNELS * (PCM_SAMPLE_WIDTH_BITS / 8);
pub const PCM_BYTE_RATE: u32 = PCM_SAMPLE_RATE_HZ * PCM_BLOCK_ALIGN as u32;
pub const WAV_HEADER_BYTES: usize = 44;
pub const WAV_FMT_CHUNK_BYTES: u32 = 16;
pub const WAV_AUDIO_FORMAT_PCM: u16 = 1;
pub const ASR_OPENAI_TRANSCRIPTION_ENDPOINT: &str =
    "https://api.openai.com/v1/audio/transcriptions";
pub const ASR_DEFAULT_MODEL: &str = "gpt-4o-transcribe";
pub const ASR_DEFAULT_MAX_UPLOAD_BYTES: u64 = 26214400;
pub const ASR_REQUEST_TIMEOUT_SECONDS: u64 = 120;
