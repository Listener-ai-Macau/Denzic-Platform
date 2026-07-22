#!/usr/bin/env python3
import argparse
import json
import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SPEC_PATH = ROOT / "ble_windows" / "protocol" / "ble_windows_v1.json"
RUST_PATH = ROOT / "ble_windows" / "host" / "rust" / "src" / "generated.rs"
C_PATH = ROOT / "ble_windows" / "embedded" / "c" / "include" / "denzic_ble_windows_v1_generated.h"

INT_CONSTS = (
    ("default_discovery_timeout_ms", "DEFAULT_DISCOVERY_TIMEOUT_MS", "u64"),
    ("optional_read_timeout_ms", "OPTIONAL_READ_TIMEOUT_MS", "u64"),
    ("cccd_enable_timeout_ms", "CCCD_ENABLE_TIMEOUT_MS", "u64"),
    ("async_poll_interval_ms", "ASYNC_POLL_INTERVAL_MS", "u64"),
    ("create_no_window_flag", "CREATE_NO_WINDOW_FLAG", "u32"),
)


def pascal(name):
    return "".join(part[:1].upper() + part[1:] for part in name.split("_"))


def c_symbol(name):
    return re.sub(r"[^A-Za-z0-9]+", "_", name).upper()


def validate_spec(spec):
    if spec.get("name") != "denzic_ble_windows_v1" or spec.get("version") != 1:
        raise SystemExit("ble_windows v1 name/version are fixed")
    for key, _, _ in INT_CONSTS:
        value = spec.get(key)
        if not isinstance(value, int) or value <= 0:
            raise SystemExit(f"{key} must be a positive integer")
    delays = spec.get("cccd_enable_retry_delays_ms")
    if (
        not isinstance(delays, list)
        or not delays
        or any(not isinstance(item, int) or item <= 0 for item in delays)
    ):
        raise SystemExit("cccd_enable_retry_delays_ms must be a non-empty array of positive integers")
    kinds = spec.get("failure_kinds")
    if not isinstance(kinds, list) or not kinds:
        raise SystemExit("failure_kinds must be a non-empty array")
    codes = set()
    names = set()
    for item in kinds:
        name = item.get("name") if isinstance(item, dict) else None
        code = item.get("code") if isinstance(item, dict) else None
        if not isinstance(name, str) or not re.fullmatch(r"[a-z][a-z0-9_]*", name):
            raise SystemExit(f"failure_kinds has invalid name: {name!r}")
        if not isinstance(code, int) or not 0 <= code <= 255:
            raise SystemExit(f"failure_kinds.{name} must have a u8 code")
        if name in names or code in codes:
            raise SystemExit("failure_kinds contains duplicate name or code")
        names.add(name)
        codes.add(code)
    if "unknown" not in names:
        raise SystemExit("failure_kinds must include unknown")


def render_rust(spec):
    lines = [
        "// Generated from ble_windows/protocol/ble_windows_v1.json. Do not edit.",
        "use serde::{Deserialize, Serialize};",
        "",
        f'pub const CONTRACT_NAME: &str = "{spec["name"]}";',
        f'pub const CONTRACT_VERSION: u8 = {spec["version"]};',
        "",
    ]
    for key, rust_name, rust_type in INT_CONSTS:
        lines.append(f"pub const {rust_name}: {rust_type} = {spec[key]};")
    delays = spec["cccd_enable_retry_delays_ms"]
    lines.append(
        f"pub const CCCD_ENABLE_RETRY_DELAYS_MS: [u64; {len(delays)}] = [{', '.join(str(item) for item in delays)}];"
    )
    lines.append("")
    lines.extend(
        [
            "#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]",
            "#[repr(u8)]",
            '#[serde(rename_all = "camelCase")]',
            "pub enum BleFailureKind {",
        ]
    )
    for item in spec["failure_kinds"]:
        lines.append(f"    {pascal(item['name'])} = {item['code']},")
    lines.append("}")
    lines.append("")
    return "\n".join(lines)


def render_c(spec):
    lines = [
        "/* Generated from ble_windows/protocol/ble_windows_v1.json. Do not edit. */",
        "#ifndef DENZIC_BLE_WINDOWS_V1_GENERATED_H",
        "#define DENZIC_BLE_WINDOWS_V1_GENERATED_H",
        "",
        "#include <stdint.h>",
        "",
        "#ifdef __cplusplus",
        'extern "C" {',
        "#endif",
        "",
        f'#define DENZIC_BLE_WINDOWS_V1_CONTRACT_NAME "{spec["name"]}"',
        f'#define DENZIC_BLE_WINDOWS_V1_CONTRACT_VERSION ({spec["version"]}u)',
        "",
    ]
    for key, rust_name, _ in INT_CONSTS:
        lines.append(f"#define DENZIC_BLE_WINDOWS_V1_{rust_name} ({spec[key]}u)")
    delays = spec["cccd_enable_retry_delays_ms"]
    lines.append(f"#define DENZIC_BLE_WINDOWS_V1_CCCD_ENABLE_RETRY_DELAYS_COUNT ({len(delays)}u)")
    lines.append("")
    lines.append("typedef enum {")
    for item in spec["failure_kinds"]:
        lines.append(
            f"    DENZIC_BLE_WINDOWS_V1_FAILURE_KIND_{c_symbol(item['name'])} = {item['code']},"
        )
    lines.append("} denzic_ble_windows_v1_failure_kind_t;")
    lines.extend(
        [
            "",
            "static const uint32_t denzic_ble_windows_v1_cccd_enable_retry_delays_ms",
            f"[DENZIC_BLE_WINDOWS_V1_CCCD_ENABLE_RETRY_DELAYS_COUNT] = {{{', '.join(str(item) + 'u' for item in delays)}}};",
            "",
            "#ifdef __cplusplus",
            "}",
            "#endif",
            "",
            "#endif",
            "",
        ]
    )
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
    validate_spec(spec)
    write_or_check(RUST_PATH, render_rust(spec), args.check)
    write_or_check(C_PATH, render_c(spec), args.check)


if __name__ == "__main__":
    main()
