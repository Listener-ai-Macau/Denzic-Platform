//! OpenAI-compatible batch ASR client (`POST /v1/audio/transcriptions`,
//! multipart form with `model` / `response_format` / `file`).
//!
//! Ported from the Companion-Type reference implementation; the error
//! message wording is part of the contract because products surface it
//! verbatim.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use reqwest::blocking::multipart;

use crate::asr::{
    extract_text_from_asr_body, normalize_transcript, AsrError, AsrErrorKind, AsrTranscript,
    BatchAsrProvider,
};
use crate::{
    ASR_DEFAULT_MAX_UPLOAD_BYTES, ASR_DEFAULT_MODEL, ASR_OPENAI_TRANSCRIPTION_ENDPOINT,
    ASR_REQUEST_TIMEOUT_SECONDS,
};

#[derive(Clone, Debug)]
pub struct OpenAiAsrConfig {
    pub endpoint: String,
    pub api_key: String,
    pub model: String,
    pub max_upload_bytes: u64,
    pub timeout: Duration,
}

impl OpenAiAsrConfig {
    pub fn new(endpoint: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            api_key: api_key.into(),
            model: ASR_DEFAULT_MODEL.to_string(),
            max_upload_bytes: ASR_DEFAULT_MAX_UPLOAD_BYTES,
            timeout: Duration::from_secs(ASR_REQUEST_TIMEOUT_SECONDS),
        }
    }
}

impl Default for OpenAiAsrConfig {
    fn default() -> Self {
        Self::new(ASR_OPENAI_TRANSCRIPTION_ENDPOINT, "")
    }
}

pub struct OpenAiCompatibleAsr {
    config: OpenAiAsrConfig,
}

impl OpenAiCompatibleAsr {
    pub const PROVIDER_ID: &'static str = "openai-compatible";

    pub fn new(config: OpenAiAsrConfig) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &OpenAiAsrConfig {
        &self.config
    }
}

impl BatchAsrProvider for OpenAiCompatibleAsr {
    fn provider_id(&self) -> &str {
        Self::PROVIDER_ID
    }

    fn transcribe_file(&self, audio_path: &Path) -> Result<AsrTranscript, AsrError> {
        let audio: PathBuf = audio_path.to_path_buf();
        let bytes = fs::metadata(&audio)
            .map_err(|err| {
                AsrError::new(
                    AsrErrorKind::InvalidAudio,
                    format!("read audio metadata {}: {err}", audio.display()),
                )
            })?
            .len();
        if bytes > self.config.max_upload_bytes {
            return Err(AsrError::new(
                AsrErrorKind::InvalidAudio,
                format!(
                    "audio file is {bytes} bytes, over ASR upload limit {}; use the host chunked long-meeting path",
                    self.config.max_upload_bytes
                ),
            ));
        }

        let form = multipart::Form::new()
            .text("model", self.config.model.clone())
            .text("response_format", "json")
            .file("file", &audio)
            .map_err(|err| {
                AsrError::new(
                    AsrErrorKind::InvalidAudio,
                    format!("attach audio file {}: {err}", audio.display()),
                )
            })?;
        let response = reqwest::blocking::Client::builder()
            .timeout(self.config.timeout)
            .build()
            .map_err(|err| {
                AsrError::new(
                    AsrErrorKind::Unavailable,
                    format!("build ASR HTTP client: {err}"),
                )
            })?
            .post(&self.config.endpoint)
            .bearer_auth(&self.config.api_key)
            .multipart(form)
            .send()
            .map_err(|err| {
                AsrError::retryable(
                    AsrErrorKind::Network,
                    format!("send ASR request to {}: {err}", self.config.endpoint),
                )
            })?;
        let status = response.status();
        let body = response.text().map_err(|err| {
            AsrError::new(
                AsrErrorKind::Provider,
                format!("read ASR response body: {err}"),
            )
        })?;
        if !status.is_success() {
            let message = format!("ASR provider returned {status}: {}", body.trim());
            return Err(match status.as_u16() {
                401 | 403 => AsrError::new(AsrErrorKind::Auth, message),
                429 => AsrError::retryable(AsrErrorKind::RateLimited, message),
                500..=599 => AsrError::retryable(AsrErrorKind::Provider, message),
                _ => AsrError::new(AsrErrorKind::Provider, message),
            });
        }
        let text = normalize_transcript(&extract_text_from_asr_body(&body));
        if text.is_empty() {
            return Err(AsrError::new(
                AsrErrorKind::Provider,
                "ASR provider returned no transcript text",
            ));
        }
        Ok(AsrTranscript {
            text,
            is_final: true,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::thread;

    fn spawn_test_server(
        body: &'static str,
        status: &'static str,
    ) -> (String, thread::JoinHandle<String>) {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let endpoint = format!(
            "http://{}/v1/audio/transcriptions",
            listener.local_addr().unwrap()
        );
        let server = thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut request = Vec::new();
            let mut chunk = [0_u8; 4096];
            loop {
                let read = stream.read(&mut chunk).unwrap_or(0);
                if read == 0 {
                    break;
                }
                request.extend_from_slice(&chunk[..read]);
                if request_is_complete(&request) {
                    break;
                }
            }
            let request_text = String::from_utf8_lossy(&request).to_string();
            let response = format!(
                "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            stream.write_all(response.as_bytes()).unwrap();
            request_text
        });
        (endpoint, server)
    }

    fn request_is_complete(request: &[u8]) -> bool {
        let text = String::from_utf8_lossy(request);
        let Some(header_end) = text.find("\r\n\r\n") else {
            return false;
        };
        let content_length = text
            .lines()
            .find_map(|line| {
                let (name, value) = line.split_once(':')?;
                if name.eq_ignore_ascii_case("content-length") {
                    value.trim().parse::<usize>().ok()
                } else {
                    None
                }
            })
            .unwrap_or(0);
        request.len() >= header_end + 4 + content_length
    }

    #[test]
    fn posts_audio_multipart_and_extracts_text() {
        let root = std::env::temp_dir().join("denzic-host-audio-openai-test");
        let _ = fs::create_dir_all(&root);
        let audio = root.join("meeting.wav");
        fs::write(&audio, b"RIFF----WAVEfmt test audio").unwrap();

        let (endpoint, server) = spawn_test_server(
            "{\"text\":\"Decision: use the platform ASR client.\"}",
            "200 OK",
        );
        let mut config = OpenAiAsrConfig::new(endpoint, "test-key");
        config.model = "gpt-4o-transcribe".to_string();
        let provider = OpenAiCompatibleAsr::new(config);

        let transcript = provider.transcribe_file(&audio).unwrap();
        assert!(transcript.is_final);
        assert!(transcript.text.contains("platform ASR client"));

        let captured = server.join().unwrap();
        let captured_lower = captured.to_ascii_lowercase();
        assert!(captured.contains("POST /v1/audio/transcriptions"));
        assert!(captured_lower.contains("authorization: bearer test-key"));
        assert!(captured.contains("name=\"model\""));
        assert!(captured.contains("gpt-4o-transcribe"));
        assert!(captured.contains("name=\"file\""));
        assert!(captured.contains("multipart/form-data"));
    }

    #[test]
    fn classifies_http_failures() {
        let root = std::env::temp_dir().join("denzic-host-audio-openai-error-test");
        let _ = fs::create_dir_all(&root);
        let audio = root.join("meeting.wav");
        fs::write(&audio, b"RIFF----WAVEfmt test audio").unwrap();

        let (endpoint, server) = spawn_test_server("{\"error\":\"bad key\"}", "401 Unauthorized");
        let provider = OpenAiCompatibleAsr::new(OpenAiAsrConfig::new(endpoint, "bad-key"));
        let err = provider.transcribe_file(&audio).unwrap_err();
        assert_eq!(err.kind, AsrErrorKind::Auth);
        assert!(!err.retryable);
        assert!(err
            .to_string()
            .starts_with("ASR provider returned 401 Unauthorized"));
        server.join().unwrap();
    }

    #[test]
    fn rejects_oversized_audio_before_upload() {
        let root = std::env::temp_dir().join("denzic-host-audio-openai-size-test");
        let _ = fs::create_dir_all(&root);
        let audio = root.join("big.wav");
        fs::write(&audio, vec![0u8; 1024]).unwrap();
        let mut config = OpenAiAsrConfig::new("http://127.0.0.1:1/unused", "key");
        config.max_upload_bytes = 512;
        let provider = OpenAiCompatibleAsr::new(config);
        let err = provider.transcribe_file(&audio).unwrap_err();
        assert_eq!(err.kind, AsrErrorKind::InvalidAudio);
        assert!(err
            .to_string()
            .contains("over ASR upload limit 512; use the host chunked long-meeting path"));
    }
}
