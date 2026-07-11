#!/usr/bin/env python3
import argparse
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SPEC_PATH = ROOT / "ota" / "protocol" / "ota_v1.json"
RUST_PATH = ROOT / "ota" / "host" / "rust" / "src" / "generated.rs"
C_PATH = ROOT / "ota" / "embedded" / "c" / "include" / "denzic_ota_v1_generated.h"


def upper_items(items):
    return [(name.upper(), value) for name, value in items.items()]


def render_rust(spec):
    lines = [
        "// Generated from ota/protocol/ota_v1.json. Do not edit.",
        f'pub const PROTOCOL_NAME: &str = "{spec["name"]}";',
        f'pub const MAGIC: [u8; 4] = *b"{spec["magic_ascii"]}";',
        f'pub const PROTOCOL_VERSION: u8 = {spec["version"]};',
        f'pub const CONTROL_BYTES: usize = {spec["control_bytes"]};',
        f'pub const DATA_HEADER_BYTES: usize = {spec["data_header_bytes"]};',
        f'pub const STATUS_BYTES: usize = {spec["status_bytes"]};',
    ]
    for group, prefix in (
        ("status_flags", "STATUS_FLAG"),
        ("operations", "OP"),
        ("states", "STATE"),
        ("errors", "ERROR"),
    ):
        for name, value in upper_items(spec[group]):
            lines.append(f"pub const {prefix}_{name}: u8 = {value};")
    return "\n".join(lines) + "\n"


def render_c(spec):
    lines = [
        "/* Generated from ota/protocol/ota_v1.json. Do not edit. */",
        "#ifndef DENZIC_OTA_V1_GENERATED_H",
        "#define DENZIC_OTA_V1_GENERATED_H",
        "",
        f'#define DENZIC_OTA_V1_PROTOCOL_NAME "{spec["name"]}"',
        f'#define DENZIC_OTA_V1_MAGIC "{spec["magic_ascii"]}"',
        f'#define DENZIC_OTA_V1_PROTOCOL_VERSION ({spec["version"]}u)',
        f'#define DENZIC_OTA_V1_CONTROL_BYTES ({spec["control_bytes"]}u)',
        f'#define DENZIC_OTA_V1_DATA_HEADER_BYTES ({spec["data_header_bytes"]}u)',
        f'#define DENZIC_OTA_V1_STATUS_BYTES ({spec["status_bytes"]}u)',
    ]
    for group, prefix in (
        ("status_flags", "STATUS_FLAG"),
        ("operations", "OP"),
        ("states", "STATE"),
        ("errors", "ERROR"),
    ):
        for name, value in upper_items(spec[group]):
            lines.append(f"#define DENZIC_OTA_V1_{prefix}_{name} ({value}u)")
    lines.extend(["", "#endif", ""])
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
    write_or_check(RUST_PATH, render_rust(spec), args.check)
    write_or_check(C_PATH, render_c(spec), args.check)


if __name__ == "__main__":
    main()
