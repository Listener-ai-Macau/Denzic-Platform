#!/usr/bin/env python3
import argparse
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SPEC_PATH = ROOT / "ble_pairing" / "protocol" / "ble_pairing_v1.json"
RUST_PATH = ROOT / "ble_pairing" / "host" / "rust" / "src" / "generated.rs"
C_PATH = ROOT / "ble_pairing" / "embedded" / "c" / "include" / "denzic_ble_pairing_v1_generated.h"

ENUM_GROUPS = (
    ("disconnect_classifications", "DISCONNECT"),
    ("advertising_after_disconnect", "ADVERTISING_AFTER_DISCONNECT"),
    ("identity_actions", "IDENTITY"),
    ("window_close_decisions", "WINDOW_CLOSE"),
    ("security_failure_actions", "SECURITY_FAILURE"),
    ("swift_pair_prompt_evaluations", "SWIFT_PAIR_PROMPT"),
    ("advertising_profiles", "ADV_PROFILE"),
    ("advertising_restart_decisions", "ADV_RESTART"),
    ("gatt_cache_policies", "GATT_CACHE"),
)


def render_rust(spec):
    lines = [
        "// Generated from ble_pairing/protocol/ble_pairing_v1.json. Do not edit.",
        f'pub const PROTOCOL_NAME: &str = "{spec["name"]}";',
        f'pub const PROTOCOL_VERSION: u8 = {spec["version"]};',
    ]
    for name, value in spec["timing_defaults"].items():
        lines.append(f"pub const {name.upper()}: u32 = {value};")
    for name, value in spec["hci"].items():
        lines.append(f"pub const HCI_{name.upper()}: u32 = {value};")
    for group, prefix in ENUM_GROUPS:
        for item in spec[group]:
            lines.append(f"pub const {prefix}_{item['name'].upper()}: u8 = {item['code']};")
    return "\n".join(lines) + "\n"


def render_c(spec):
    lines = [
        "/* Generated from ble_pairing/protocol/ble_pairing_v1.json. Do not edit. */",
        "#ifndef DENZIC_BLE_PAIRING_V1_GENERATED_H",
        "#define DENZIC_BLE_PAIRING_V1_GENERATED_H",
        "",
        f'#define DENZIC_BLE_PAIRING_V1_PROTOCOL_NAME "{spec["name"]}"',
        f'#define DENZIC_BLE_PAIRING_V1_PROTOCOL_VERSION ({spec["version"]}u)',
    ]
    for name, value in spec["timing_defaults"].items():
        lines.append(f"#define DENZIC_BLE_PAIRING_V1_{name.upper()} ({value}u)")
    for name, value in spec["hci"].items():
        lines.append(f"#define DENZIC_BLE_PAIRING_V1_HCI_{name.upper()} ({value}u)")
    for group, prefix in ENUM_GROUPS:
        for item in spec[group]:
            lines.append(f"#define DENZIC_BLE_PAIRING_V1_{prefix}_{item['name'].upper()} ({item['code']}u)")
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
    hci = spec["hci"]
    composed = hci["nimble_hci_status_base"] + hci["remote_user_terminated_reason"]
    if composed != hci["host_deliberate_disconnect_status"]:
        raise SystemExit("hci.host_deliberate_disconnect_status must equal base + remote_user_terminated_reason")
    write_or_check(RUST_PATH, render_rust(spec), args.check)
    write_or_check(C_PATH, render_c(spec), args.check)


if __name__ == "__main__":
    main()
