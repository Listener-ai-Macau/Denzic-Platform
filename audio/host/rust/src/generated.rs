// Generated from audio/protocol/audio_v1.json. Do not edit.
pub const PROTOCOL_NAME: &str = "denzic_audio_v1";
pub const MAGIC: &[u8; 4] = b"VKA1";
pub const MAGIC_U32: u32 = 0x31414b56;
pub const PROTOCOL_VERSION: u8 = 1;
pub const HEADER_LEN: usize = 20;
pub const PCM_SAMPLE_RATE_HZ: u32 = 16000;
pub const PCM_CHANNELS: u16 = 1;
pub const PCM_SAMPLE_WIDTH_BITS: u16 = 16;
pub const PCM_BYTES_PER_SECOND: usize =
    PCM_SAMPLE_RATE_HZ as usize * PCM_CHANNELS as usize * (PCM_SAMPLE_WIDTH_BITS as usize / 8);
pub const PACKET_TYPE_SESSION_START: u8 = 1;
pub const PACKET_TYPE_AUDIO_DATA: u8 = 2;
pub const PACKET_TYPE_SESSION_STOP: u8 = 3;
pub const PACKET_TYPE_SESSION_CANCEL: u8 = 4;
pub const PACKET_TYPE_SESSION_ERROR: u8 = 5;
pub const SESSION_ERROR_NONE: u16 = 0;
pub const SESSION_ERROR_QUEUE_FULL: u16 = 1;
pub const SESSION_ERROR_NOTIFY_TIMEOUT: u16 = 2;
pub const SESSION_ERROR_LINK_LOST: u16 = 3;
pub const SESSION_ERROR_SEQUENCE_OVERFLOW: u16 = 4;
pub const SESSION_ERROR_INVALID_STATE: u16 = 5;
pub const SESSION_ERROR_NO_MEMORY: u16 = 6;
pub const SESSION_ERROR_PACKET_TOO_LARGE: u16 = 7;
pub const SESSION_ERROR_TRANSPORT: u16 = 8;
