# Host audio capability: adapter boundary

The host-audio capability carries the host recording/ASR wire constants
(`protocol/host_audio_v1.json`: 16 kHz / mono / 16-bit little-endian PCM, the
44-byte RIFF/WAV header layout, and the OpenAI-compatible ASR defaults), the
host microphone capture engine, the WAV container core, and the ASR provider
contracts with an OpenAI-compatible batch client. This document states what
stays in the platform and what product adapters must inject.

## What the platform owns

- `host/rust/src/capture.rs` — cpal capture: input-device enumeration and
  selection (by name or system default), default-config negotiation,
  sample-format dispatch (f32 / i16 / u16), arithmetic-mean mono downmix, the
  capture thread lifecycle (startup handshake, stop flag, join), and startup
  error classification (`CaptureError`). The sink callback receives mono f32
  frames plus the device sample rate.
- `host/rust/src/wav.rs` — the 44-byte WAV header builder, in-memory WAV
  encoding, and the appending `WavWriter` that backfills RIFF/data sizes on
  finalize/drop. `embedded/c/src/denzic_host_audio_v1.c` mirrors the header
  builder for C consumers; both ends derive constants from the generated
  protocol files.
- `host/rust/src/asr.rs` — the ASR contracts: `AsrErrorKind` classification
  (invalid audio / auth / network / rate-limited / provider / unavailable /
  cancelled), the `BatchAsrProvider` trait, and the streaming trio
  (`StreamingAsrProvider` / `StreamingAsrSession` / `AsrEventSink`) with
  partial/final event separation.
- `host/rust/src/asr_openai.rs` — the OpenAI-compatible batch client
  (multipart `POST /v1/audio/transcriptions`), including upload-limit
  enforcement and HTTP-to-`AsrErrorKind` mapping.

## What product adapters own

- Session management and file placement: recording directories, file naming,
  session IDs, and start/stop orchestration (e.g. Companion-Type's
  `HostRecordingManager`).
- Resampling from the device rate to the contract 16 kHz: the two reference
  products use different resamplers (ratio accumulator vs. cross-buffer
  linear interpolation), so this stays product-side behind the capture sink.
- Provider selection policy and credential sourcing: which ASR provider runs,
  env-var / keychain plumbing, and product-specific guardrails (e.g.
  Companion-Type's sidecar/env/command chain and its listener-type command
  rejection).
- Streaming ASR providers beyond the OpenAI-compatible client: Volcengine
  SAUC and local engines remain Listener-Type adapters implementing the
  platform traits (tracked as a follow-up).
- Runtime stream-error reporting: products pass `on_stream_error` to route
  cpal runtime errors into their own diagnostics.
- Microphone permission flows: the capture engine surfaces OS errors
  verbatim; permission UX and keyword classification stay with the product.

## Wiring

- Hosts depend on the `denzic-host-audio-v1-core` crate by path from the
  pinned submodule. PCM/WAV/ASR constants come from the crate's generated
  re-exports, never re-hardcoded.
- C consumers compile `host_audio/embedded/c/src/denzic_host_audio_v1.c` and
  pin constants to `denzic_host_audio_v1_generated.h`.

`tools/verify.ps1` regenerates both ends (`generate_host_audio_v1.py
--check`) and runs the C core's ctest suite and the host crate's unit tests
(no microphone required; capture tests use synthetic PCM only).
