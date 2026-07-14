#!/usr/bin/env python3
import argparse
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SPEC_PATH = ROOT / "audio" / "protocol" / "audio_v1.json"
RUST_PATH = ROOT / "audio" / "host" / "rust" / "src" / "generated.rs"
C_PATH = ROOT / "audio" / "embedded" / "c" / "include" / "denzic_audio_v1_generated.h"


def upper_items(items):
    return [(name.upper(), value) for name, value in items.items()]


def magic_u32(value):
    return int.from_bytes(value.encode("ascii"), "little")


def render_rust(spec):
    pcm = spec["pcm"]
    lines = [
        "// Generated from audio/protocol/audio_v1.json. Do not edit.",
        f'pub const PROTOCOL_NAME: &str = "{spec["name"]}";',
        f'pub const MAGIC: &[u8; 4] = b"{spec["magic_ascii"]}";',
        f'pub const MAGIC_U32: u32 = 0x{magic_u32(spec["magic_ascii"]):08x};',
        f'pub const PROTOCOL_VERSION: u8 = {spec["version"]};',
        f'pub const HEADER_LEN: usize = {spec["header_bytes"]};',
        f'pub const PCM_SAMPLE_RATE_HZ: u32 = {pcm["sample_rate_hz"]};',
        f'pub const PCM_CHANNELS: u16 = {pcm["channels"]};',
        f'pub const PCM_SAMPLE_WIDTH_BITS: u16 = {pcm["sample_width_bits"]};',
        "pub const PCM_BYTES_PER_SECOND: usize =",
        "    PCM_SAMPLE_RATE_HZ as usize * PCM_CHANNELS as usize * (PCM_SAMPLE_WIDTH_BITS as usize / 8);",
    ]
    for name, value in upper_items(spec["packet_types"]):
        lines.append(f"pub const PACKET_TYPE_{name}: u8 = {value};")
    for name, value in upper_items(spec["session_errors"]):
        lines.append(f"pub const SESSION_ERROR_{name}: u16 = {value};")
    return "\n".join(lines) + "\n"


def render_c(spec):
    pcm = spec["pcm"]
    lines = [
        "/* Generated from audio/protocol/audio_v1.json. Do not edit. */",
        "#ifndef DENZIC_AUDIO_V1_GENERATED_H",
        "#define DENZIC_AUDIO_V1_GENERATED_H",
        "",
        "#include <stddef.h>",
        "#include <stdint.h>",
        "",
        "#ifdef __cplusplus",
        "extern \"C\" {",
        "#endif",
        "",
        f'#define DENZIC_AUDIO_V1_PROTOCOL_NAME "{spec["name"]}"',
        f'#define DENZIC_AUDIO_V1_MAGIC "{spec["magic_ascii"]}"',
        f'#define DENZIC_AUDIO_V1_MAGIC_U32 (0x{magic_u32(spec["magic_ascii"]):08x}u)',
        f'#define DENZIC_AUDIO_V1_PROTOCOL_VERSION ({spec["version"]}u)',
        f'#define DENZIC_AUDIO_V1_HEADER_BYTES ({spec["header_bytes"]}u)',
        f'#define DENZIC_AUDIO_V1_PCM_SAMPLE_RATE_HZ ({pcm["sample_rate_hz"]}u)',
        f'#define DENZIC_AUDIO_V1_PCM_CHANNELS ({pcm["channels"]}u)',
        f'#define DENZIC_AUDIO_V1_PCM_SAMPLE_WIDTH_BITS ({pcm["sample_width_bits"]}u)',
        "#define DENZIC_AUDIO_V1_PCM_BYTES_PER_SECOND (DENZIC_AUDIO_V1_PCM_SAMPLE_RATE_HZ * DENZIC_AUDIO_V1_PCM_CHANNELS * (DENZIC_AUDIO_V1_PCM_SAMPLE_WIDTH_BITS / 8u))",
        "",
        "typedef enum {",
    ]
    for name, value in upper_items(spec["packet_types"]):
        lines.append(f"    DENZIC_AUDIO_V1_PACKET_TYPE_{name} = {value},")
    lines.extend([
        "    DENZIC_AUDIO_V1_PACKET_TYPE_AUDIO_CHUNK = DENZIC_AUDIO_V1_PACKET_TYPE_AUDIO_DATA,",
        "} denzic_audio_v1_packet_type_t;",
        "",
        "typedef enum {",
    ])
    for name, value in upper_items(spec["session_errors"]):
        lines.append(f"    DENZIC_AUDIO_V1_SESSION_ERROR_{name} = {value},")
    lines.extend([
        "} denzic_audio_v1_session_error_t;",
        "",
        "#if defined(_MSC_VER)",
        "#pragma pack(push, 1)",
        "#define DENZIC_AUDIO_V1_PACKED",
        "#else",
        "#define DENZIC_AUDIO_V1_PACKED __attribute__((packed))",
        "#endif",
        "typedef struct DENZIC_AUDIO_V1_PACKED {",
        "    uint8_t magic[4];",
        "    uint8_t packet_type;",
        "    uint8_t flags;",
        "    uint16_t header_len_le;",
        "    uint32_t session_id_le;",
        "    uint16_t chunk_index_le;",
        "    uint8_t fragment_index;",
        "    uint8_t fragment_count;",
        "    uint16_t payload_len_le;",
        "    uint16_t chunk_pcm_bytes_le;",
        "} denzic_audio_v1_packet_header_t;",
        "#if defined(_MSC_VER)",
        "#pragma pack(pop)",
        "#endif",
        "",
        "static inline void denzic_audio_v1_packet_header_init(",
        "    denzic_audio_v1_packet_header_t *header,",
        "    denzic_audio_v1_packet_type_t packet_type,",
        "    uint32_t session_id,",
        "    uint16_t chunk_index,",
        "    uint8_t fragment_index,",
        "    uint8_t fragment_count,",
        "    uint16_t payload_len,",
        "    uint16_t chunk_pcm_bytes)",
        "{",
        "    if (header == NULL) {",
        "        return;",
        "    }",
        "    header->magic[0] = 'V';",
        "    header->magic[1] = 'K';",
        "    header->magic[2] = 'A';",
        "    header->magic[3] = '1';",
        "    header->packet_type = (uint8_t)packet_type;",
        "    header->flags = 0u;",
        "    header->header_len_le = DENZIC_AUDIO_V1_HEADER_BYTES;",
        "    header->session_id_le = session_id;",
        "    header->chunk_index_le = chunk_index;",
        "    header->fragment_index = fragment_index;",
        "    header->fragment_count = fragment_count;",
        "    header->payload_len_le = payload_len;",
        "    header->chunk_pcm_bytes_le = chunk_pcm_bytes;",
        "}",
        "",
        "#ifdef __cplusplus",
        "}",
        "#endif",
        "",
        "#endif",
        "",
    ])
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
    if len(spec["magic_ascii"].encode("ascii")) != 4:
        raise SystemExit("magic_ascii must be exactly four ASCII bytes")
    if spec["header_bytes"] != 20:
        raise SystemExit("audio v1 header_bytes must remain 20")
    if spec["pcm"]["sample_rate_hz"] <= 0 or spec["pcm"]["channels"] <= 0:
        raise SystemExit("audio PCM sample rate and channels must be positive")
    write_or_check(RUST_PATH, render_rust(spec), args.check)
    write_or_check(C_PATH, render_c(spec), args.check)


if __name__ == "__main__":
    main()
