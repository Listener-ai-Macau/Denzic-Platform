//! ASR provider contracts: error classification, batch and streaming
//! provider traits, and transcript text helpers shared by host products.

use std::fmt;
use std::path::Path;
use std::sync::Arc;

/// Coarse error classification so products can map provider failures onto
/// their own UX (retry prompts, permission guidance, fallback providers)
/// without parsing message strings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AsrErrorKind {
    /// Input audio is missing, unreadable, or violates provider limits.
    InvalidAudio,
    /// Credentials missing or rejected (HTTP 401/403).
    Auth,
    /// Transport-level failure (DNS, connect, TLS, timeout).
    Network,
    /// HTTP 429; retry after backoff.
    RateLimited,
    /// Provider returned an error or an unusable response.
    Provider,
    /// Provider is not configured or not available in this environment.
    Unavailable,
    /// The session was cancelled by the caller.
    Cancelled,
}

#[derive(Debug, Clone)]
pub struct AsrError {
    pub kind: AsrErrorKind,
    pub message: String,
    pub retryable: bool,
}

impl AsrError {
    pub fn new(kind: AsrErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            retryable: false,
        }
    }

    pub fn retryable(kind: AsrErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
            retryable: true,
        }
    }
}

impl fmt::Display for AsrError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for AsrError {}

/// Transcript produced by a batch transcription or a finished stream.
#[derive(Debug, Clone)]
pub struct AsrTranscript {
    pub text: String,
    pub is_final: bool,
}

/// Batch (file-based) ASR provider.
pub trait BatchAsrProvider: Send + Sync {
    fn provider_id(&self) -> &str;
    /// Transcribe a complete audio file (WAV in the contract format unless
    /// the provider documents otherwise).
    fn transcribe_file(&self, audio_path: &Path) -> Result<AsrTranscript, AsrError>;
}

/// Sink receiving streaming ASR events from a [`StreamingAsrSession`].
pub trait AsrEventSink: Send + Sync {
    /// Interim hypothesis; may be revised by later partials.
    fn on_partial(&self, text: &str);
    /// Finalized segment.
    fn on_final(&self, text: &str);
    fn on_error(&self, error: &AsrError);
}

/// A live streaming ASR session. PCM pushed here must be in the contract
/// format (16 kHz / mono / 16-bit little-endian).
pub trait StreamingAsrSession: Send {
    fn push_pcm(&self, pcm: &[u8]);
    /// Close the audio stream and return the accumulated final transcript.
    fn finish(&self) -> Result<AsrTranscript, AsrError>;
    fn cancel(&self);
}

/// Streaming ASR provider: creates one session per recording.
pub trait StreamingAsrProvider: Send + Sync {
    fn provider_id(&self) -> &str;
    fn start_session(
        &self,
        sink: Arc<dyn AsrEventSink>,
    ) -> Result<Box<dyn StreamingAsrSession>, AsrError>;
}

/// Normalize raw transcript text: CRLF -> LF, trim lines, drop empty lines.
pub fn normalize_transcript(text: &str) -> String {
    text.replace("\r\n", "\n")
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Extract transcript text from an OpenAI-compatible response body:
/// `{"text": ...}`, `{"transcript": {"text": ...}}`, or raw text.
pub fn extract_text_from_asr_body(body: &str) -> String {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(body) {
        if let Some(text) = value.get("text").and_then(serde_json::Value::as_str) {
            return text.to_string();
        }
        if let Some(text) = value
            .get("transcript")
            .and_then(|transcript| transcript.get("text"))
            .and_then(serde_json::Value::as_str)
        {
            return text.to_string();
        }
    }
    body.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_transcript_trims_and_drops_empty_lines() {
        assert_eq!(
            normalize_transcript("  Decision: ship it.\r\n\r\nAction: verify.  \n"),
            "Decision: ship it.\nAction: verify."
        );
        assert_eq!(normalize_transcript(" \n \n"), "");
    }

    #[test]
    fn extract_text_handles_json_and_raw_bodies() {
        assert_eq!(extract_text_from_asr_body("{\"text\":\"hello\"}"), "hello");
        assert_eq!(
            extract_text_from_asr_body("{\"transcript\":{\"text\":\"nested\"}}"),
            "nested"
        );
        assert_eq!(extract_text_from_asr_body("  raw text  "), "raw text");
    }

    #[test]
    fn asr_error_displays_its_message() {
        let err = AsrError::retryable(AsrErrorKind::Network, "connect failed");
        assert_eq!(err.to_string(), "connect failed");
        assert!(err.retryable);
        assert_eq!(err.kind, AsrErrorKind::Network);
    }
}
