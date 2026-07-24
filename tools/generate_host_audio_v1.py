#!/usr/bin/env python3
import argparse
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SPEC_PATH = ROOT / "host_audio" / "protocol" / "host_audio_v1.json"
RUST_PATH = ROOT / "host_audio" / "host" / "rust" / "src" / "generated.rs"
C_PATH = ROOT / "host_audio" / "embedded" / "c" / "include" / "denzic_host_audio_v1_generated.h"


def render_rust(spec):
    pcm = spec["pcm"]
    wav = spec["wav"]
    asr = spec["asr"]
    lines = [
        "// Generated from host_audio/protocol/host_audio_v1.json. Do not edit.",
        f'pub const CONTRACT_NAME: &str = "{spec["name"]}";',
        f'pub const CONTRACT_VERSION: u8 = {spec["version"]};',
        f'pub const PCM_SAMPLE_RATE_HZ: u32 = {pcm["sample_rate_hz"]};',
        f'pub const PCM_CHANNELS: u16 = {pcm["channels"]};',
        f'pub const PCM_SAMPLE_WIDTH_BITS: u16 = {pcm["sample_width_bits"]};',
        "pub const PCM_BLOCK_ALIGN: u16 = PCM_CHANNELS * (PCM_SAMPLE_WIDTH_BITS / 8);",
        "pub const PCM_BYTE_RATE: u32 = PCM_SAMPLE_RATE_HZ * PCM_BLOCK_ALIGN as u32;",
        f'pub const WAV_HEADER_BYTES: usize = {wav["header_bytes"]};',
        f'pub const WAV_FMT_CHUNK_BYTES: u32 = {wav["fmt_chunk_bytes"]};',
        f'pub const WAV_AUDIO_FORMAT_PCM: u16 = {wav["audio_format_pcm"]};',
        f'pub const ASR_OPENAI_TRANSCRIPTION_ENDPOINT: &str =',
        f'    "{asr["openai_transcription_endpoint"]}";',
        f'pub const ASR_DEFAULT_MODEL: &str = "{asr["default_model"]}";',
        f'pub const ASR_DEFAULT_MAX_UPLOAD_BYTES: u64 = {asr["default_max_upload_bytes"]};',
        f'pub const ASR_REQUEST_TIMEOUT_SECONDS: u64 = {asr["request_timeout_seconds"]};',
    ]
    return "\n".join(lines) + "\n"


def render_c(spec):
    pcm = spec["pcm"]
    wav = spec["wav"]
    asr = spec["asr"]
    lines = [
        "/* Generated from host_audio/protocol/host_audio_v1.json. Do not edit. */",
        "#ifndef DENZIC_HOST_AUDIO_V1_GENERATED_H",
        "#define DENZIC_HOST_AUDIO_V1_GENERATED_H",
        "",
        "#include <stdint.h>",
        "",
        "#ifdef __cplusplus",
        'extern "C" {',
        "#endif",
        "",
        f'#define DENZIC_HOST_AUDIO_V1_CONTRACT_NAME "{spec["name"]}"',
        f'#define DENZIC_HOST_AUDIO_V1_CONTRACT_VERSION ({spec["version"]}u)',
        f'#define DENZIC_HOST_AUDIO_V1_PCM_SAMPLE_RATE_HZ ({pcm["sample_rate_hz"]}u)',
        f'#define DENZIC_HOST_AUDIO_V1_PCM_CHANNELS ({pcm["channels"]}u)',
        f'#define DENZIC_HOST_AUDIO_V1_PCM_SAMPLE_WIDTH_BITS ({pcm["sample_width_bits"]}u)',
        "#define DENZIC_HOST_AUDIO_V1_PCM_BLOCK_ALIGN (DENZIC_HOST_AUDIO_V1_PCM_CHANNELS * (DENZIC_HOST_AUDIO_V1_PCM_SAMPLE_WIDTH_BITS / 8u))",
        "#define DENZIC_HOST_AUDIO_V1_PCM_BYTE_RATE (DENZIC_HOST_AUDIO_V1_PCM_SAMPLE_RATE_HZ * DENZIC_HOST_AUDIO_V1_PCM_BLOCK_ALIGN)",
        f'#define DENZIC_HOST_AUDIO_V1_WAV_HEADER_BYTES ({wav["header_bytes"]}u)',
        f'#define DENZIC_HOST_AUDIO_V1_WAV_FMT_CHUNK_BYTES ({wav["fmt_chunk_bytes"]}u)',
        f'#define DENZIC_HOST_AUDIO_V1_WAV_AUDIO_FORMAT_PCM ({wav["audio_format_pcm"]}u)',
        f'#define DENZIC_HOST_AUDIO_V1_ASR_OPENAI_TRANSCRIPTION_ENDPOINT "{asr["openai_transcription_endpoint"]}"',
        f'#define DENZIC_HOST_AUDIO_V1_ASR_DEFAULT_MODEL "{asr["default_model"]}"',
        f'#define DENZIC_HOST_AUDIO_V1_ASR_DEFAULT_MAX_UPLOAD_BYTES ({asr["default_max_upload_bytes"]}ull)',
        f'#define DENZIC_HOST_AUDIO_V1_ASR_REQUEST_TIMEOUT_SECONDS ({asr["request_timeout_seconds"]}u)',
        "",
        "#ifdef __cplusplus",
        "}",
        "#endif",
        "",
        "#endif",
        "",
    ]
    return "\n".join(lines)


def write_or_check(path, expected, check):
    if check:
        actual = path.read_text(encoding="utf-8") if path.exists() else None
        if actual != expected:
            raise SystemExit(f"generated file is stale: {path.relative_to(ROOT)}")
        return
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(expected, encoding="utf-8", newline="\n")


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    spec = json.loads(SPEC_PATH.read_text(encoding="utf-8"))
    pcm = spec["pcm"]
    if pcm["sample_rate_hz"] != 16000:
        raise SystemExit("host audio PCM sample rate must remain 16000 Hz")
    if pcm["channels"] != 1 or pcm["sample_width_bits"] != 16:
        raise SystemExit("host audio PCM format must remain mono 16-bit")
    if pcm["sample_byte_order"] != "little_endian":
        raise SystemExit("host audio PCM byte order must remain little_endian")
    if spec["wav"]["header_bytes"] != 44 or spec["wav"]["fmt_chunk_bytes"] != 16:
        raise SystemExit("host audio WAV header must remain the 44-byte PCM layout")
    write_or_check(RUST_PATH, render_rust(spec), args.check)
    write_or_check(C_PATH, render_c(spec), args.check)


if __name__ == "__main__":
    main()
