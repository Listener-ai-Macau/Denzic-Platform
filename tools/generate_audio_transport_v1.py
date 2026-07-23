#!/usr/bin/env python3
import argparse
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SPEC_PATH = ROOT / "audio" / "protocol" / "audio_transport_v1.json"
RUST_PATH = ROOT / "audio" / "host" / "rust" / "src" / "generated_transport.rs"
C_PATH = ROOT / "audio" / "embedded" / "c" / "include" / "denzic_audio_transport_v1_generated.h"


def render_rust(spec):
    media = spec["media_clock"]
    pacing = spec["pacing"]
    session = spec["fixed_session"]
    replay = spec["replay"]
    capacity = spec["capacity"]
    backpressure = spec["backpressure"]
    packet = spec["packet"]
    lines = [
        "// Generated from audio/protocol/audio_transport_v1.json. Do not edit.",
        f'pub const TRANSPORT_PROTOCOL_NAME: &str = "{spec["name"]}";',
        f'pub const TRANSPORT_PROTOCOL_VERSION: u8 = {spec["version"]};',
        f'pub const TRANSPORT_PCM_BYTES_PER_SECOND: u32 = {media["pcm_bytes_per_second"]};',
        f'pub const PACING_TARGET_BYTES_PER_SECOND: u32 = {pacing["target_bytes_per_second"]};',
        f'pub const PACING_TICK_MS: u32 = {pacing["tick_ms"]};',
        f'pub const PACING_BYTES_PER_TICK: u32 = {pacing["bytes_per_tick"]};',
        f'pub const FIXED_SESSION_DURATION_SECONDS: u32 = {session["duration_seconds"]};',
        f'pub const FIXED_SESSION_PCM_BYTES: u32 = {session["pcm_bytes"]};',
        f'pub const REPLAY_WINDOW_PACKETS: usize = {replay["window_packets"]};',
        f'pub const REPLAY_PAYLOAD_BYTES: usize = {replay["payload_bytes"]};',
        f'pub const NOTIFY_QUEUE_LENGTH_DEFAULT: u32 = {capacity["notify_queue_length_default"]};',
        f'pub const NOTIFY_QUEUE_LENGTH_SPIRAM: u32 = {capacity["notify_queue_length_spiram"]};',
        f'pub const AUDIO_POOL_EXTRA_DEFAULT: u32 = {capacity["audio_pool_extra_default"]};',
        f'pub const AUDIO_POOL_EXTRA_SPIRAM: u32 = {capacity["audio_pool_extra_spiram"]};',
        f'pub const AUDIO_POOL_BUFFER_BYTES: u32 = {capacity["audio_pool_buffer_bytes"]};',
        f'pub const BACKPRESSURE_PAUSE_PERCENT: u32 = {backpressure["pause_percent"]};',
        f'pub const BACKPRESSURE_RESUME_PERCENT: u32 = {backpressure["resume_percent"]};',
        f'pub const AUDIO_POOL_WARN_PERCENT: u32 = {backpressure["pool_warn_percent"]};',
        f'pub const PACKET_DEFAULT_VALUE_MAX_BYTES: u16 = {packet["default_value_max_bytes"]};',
        f'pub const PACKET_MAX_VALUE_BYTES: u16 = {packet["max_value_bytes"]};',
        f'pub const PACKET_MTU_ATT_OVERHEAD_BYTES: u16 = {packet["mtu_att_overhead_bytes"]};',
    ]
    return "\n".join(lines) + "\n"


def render_c(spec):
    media = spec["media_clock"]
    pacing = spec["pacing"]
    session = spec["fixed_session"]
    replay = spec["replay"]
    capacity = spec["capacity"]
    backpressure = spec["backpressure"]
    packet = spec["packet"]
    lines = [
        "/* Generated from audio/protocol/audio_transport_v1.json. Do not edit. */",
        "#ifndef DENZIC_AUDIO_TRANSPORT_V1_GENERATED_H",
        "#define DENZIC_AUDIO_TRANSPORT_V1_GENERATED_H",
        "",
        f'#define DENZIC_AUDIO_TRANSPORT_V1_PROTOCOL_NAME "{spec["name"]}"',
        f'#define DENZIC_AUDIO_TRANSPORT_V1_PROTOCOL_VERSION ({spec["version"]}u)',
        f'#define DENZIC_AUDIO_TRANSPORT_V1_PCM_BYTES_PER_SECOND ({media["pcm_bytes_per_second"]}u)',
        f'#define DENZIC_AUDIO_TRANSPORT_V1_PACING_TARGET_BYTES_PER_SECOND ({pacing["target_bytes_per_second"]}u)',
        f'#define DENZIC_AUDIO_TRANSPORT_V1_PACING_TICK_MS ({pacing["tick_ms"]}u)',
        f'#define DENZIC_AUDIO_TRANSPORT_V1_PACING_BYTES_PER_TICK ({pacing["bytes_per_tick"]}u)',
        f'#define DENZIC_AUDIO_TRANSPORT_V1_FIXED_SESSION_DURATION_SECONDS ({session["duration_seconds"]}u)',
        f'#define DENZIC_AUDIO_TRANSPORT_V1_FIXED_SESSION_PCM_BYTES ({session["pcm_bytes"]}u)',
        f'#define DENZIC_AUDIO_TRANSPORT_V1_REPLAY_WINDOW_PACKETS ({replay["window_packets"]}u)',
        f'#define DENZIC_AUDIO_TRANSPORT_V1_REPLAY_PAYLOAD_BYTES ({replay["payload_bytes"]}u)',
        f'#define DENZIC_AUDIO_TRANSPORT_V1_NOTIFY_QUEUE_LENGTH_DEFAULT ({capacity["notify_queue_length_default"]}u)',
        f'#define DENZIC_AUDIO_TRANSPORT_V1_NOTIFY_QUEUE_LENGTH_SPIRAM ({capacity["notify_queue_length_spiram"]}u)',
        f'#define DENZIC_AUDIO_TRANSPORT_V1_AUDIO_POOL_EXTRA_DEFAULT ({capacity["audio_pool_extra_default"]}u)',
        f'#define DENZIC_AUDIO_TRANSPORT_V1_AUDIO_POOL_EXTRA_SPIRAM ({capacity["audio_pool_extra_spiram"]}u)',
        f'#define DENZIC_AUDIO_TRANSPORT_V1_AUDIO_POOL_BUFFER_BYTES ({capacity["audio_pool_buffer_bytes"]}u)',
        f'#define DENZIC_AUDIO_TRANSPORT_V1_BACKPRESSURE_PAUSE_PERCENT ({backpressure["pause_percent"]}u)',
        f'#define DENZIC_AUDIO_TRANSPORT_V1_BACKPRESSURE_RESUME_PERCENT ({backpressure["resume_percent"]}u)',
        f'#define DENZIC_AUDIO_TRANSPORT_V1_AUDIO_POOL_WARN_PERCENT ({backpressure["pool_warn_percent"]}u)',
        f'#define DENZIC_AUDIO_TRANSPORT_V1_PACKET_DEFAULT_VALUE_MAX_BYTES ({packet["default_value_max_bytes"]}u)',
        f'#define DENZIC_AUDIO_TRANSPORT_V1_PACKET_MAX_VALUE_BYTES ({packet["max_value_bytes"]}u)',
        f'#define DENZIC_AUDIO_TRANSPORT_V1_PACKET_MTU_ATT_OVERHEAD_BYTES ({packet["mtu_att_overhead_bytes"]}u)',
        "",
        "#endif",
        "",
    ]
    return "\n".join(lines)


def validate(spec):
    if spec["base_contract"] != "denzic_audio_v1":
        raise SystemExit("audio transport v1 must layer on the denzic_audio_v1 wire contract")
    pacing = spec["pacing"]
    if pacing["target_bytes_per_second"] != 38400:
        raise SystemExit("wire pacing target must remain 38400 B/s (flow_control_v1.md)")
    if pacing["tick_ms"] != 10:
        raise SystemExit("pacing tick must remain 10 ms (flow_control_v1.md)")
    derived = pacing["target_bytes_per_second"] * pacing["tick_ms"] // 1000
    if pacing["bytes_per_tick"] != derived:
        raise SystemExit("pacing bytes_per_tick must equal target*tick_ms/1000")
    session = spec["fixed_session"]
    if session["duration_seconds"] != 60:
        raise SystemExit("fixed session duration must remain 60 s (flow_control_v1.md)")
    if session["pcm_bytes"] != session["duration_seconds"] * spec["media_clock"]["pcm_bytes_per_second"]:
        raise SystemExit("fixed session pcm_bytes must equal duration * pcm byte rate")
    replay = spec["replay"]
    if replay["window_packets"] != 48:
        raise SystemExit("replay retained window must remain 48 packets (flow_control_v1.md)")
    packet = spec["packet"]
    if replay["payload_bytes"] != packet["max_value_bytes"] - 20:
        raise SystemExit("replay payload bytes must equal max packet value bytes minus the 20-byte VKA1 header")
    backpressure = spec["backpressure"]
    if backpressure["pause_percent"] != 95 or backpressure["resume_percent"] != 70:
        raise SystemExit("backpressure hysteresis must remain pause=95% resume=70% (flow_control_v1.md)")
    if backpressure["pool_warn_percent"] != 80:
        raise SystemExit("audio pool pressure warning must remain 80% (flow_control_v1.md)")
    capacity = spec["capacity"]
    if capacity["notify_queue_length_default"] != 48 or capacity["notify_queue_length_spiram"] != 256:
        raise SystemExit("notify queue lengths must remain 48 (no SPIRAM) / 256 (SPIRAM)")
    if capacity["audio_pool_extra_default"] != 4 or capacity["audio_pool_extra_spiram"] != 8:
        raise SystemExit("audio pool extra buffers must remain 4 (no SPIRAM) / 8 (SPIRAM)")


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
    validate(spec)
    write_or_check(RUST_PATH, render_rust(spec), args.check)
    write_or_check(C_PATH, render_c(spec), args.check)


if __name__ == "__main__":
    main()
