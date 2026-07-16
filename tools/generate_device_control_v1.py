#!/usr/bin/env python3
"""Generate the shared device-control v1 enum surface from its contract."""

import argparse
import json
import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SPEC_PATH = ROOT / "device_control" / "protocol" / "device_control_v1.json"
RUST_PATH = ROOT / "device_control" / "host" / "rust" / "src" / "generated.rs"
C_PATH = (
    ROOT
    / "device_control"
    / "embedded"
    / "c"
    / "include"
    / "denzic_device_control_v1_generated.h"
)

ENUMS = (
    ("transports", "Transport", "transport"),
    ("lifecycle_states", "LifecycleState", "lifecycle_state"),
    ("ownership_states", "OwnershipState", "ownership_state"),
    ("operation_kinds", "OperationKind", "operation_kind"),
    ("operation_results", "OperationResult", "operation_result"),
    ("error_categories", "ErrorCategory", "error_category"),
)


def pascal(name):
    return "".join(part[:1].upper() + part[1:] for part in name.split("_"))


def c_symbol(name):
    return re.sub(r"[^A-Za-z0-9]+", "_", name).upper()


def validate_spec(spec):
    if spec.get("name") != "denzic_device_control_v1" or spec.get("version") != 1:
        raise SystemExit("device-control v1 name/version are fixed")
    for key, _, _ in ENUMS:
        values = spec.get(key)
        if not isinstance(values, list) or not values:
            raise SystemExit(f"{key} must be a non-empty array")
        names = set()
        codes = set()
        for item in values:
            name = item.get("name") if isinstance(item, dict) else None
            code = item.get("code") if isinstance(item, dict) else None
            if not isinstance(name, str) or not re.fullmatch(r"[a-z][a-z0-9_]*", name):
                raise SystemExit(f"{key} has invalid name: {name!r}")
            if not isinstance(code, int) or not 0 <= code <= 255:
                raise SystemExit(f"{key}.{name} must have a u8 code")
            if name in names or code in codes:
                raise SystemExit(f"{key} contains duplicate name or code")
            names.add(name)
            codes.add(code)
    for key in ("transports", "lifecycle_states", "ownership_states", "operation_kinds"):
        if "unknown" not in {item["name"] for item in spec[key]}:
            raise SystemExit(f"{key} must include unknown")
    if "none" not in {item["name"] for item in spec["error_categories"]}:
        raise SystemExit("error_categories must include none")
    if "accepted" not in {item["name"] for item in spec["operation_results"]}:
        raise SystemExit("operation_results must include accepted")


def render_rust_enum(items, rust_type):
    lines = [
        "#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]",
        "#[repr(u8)]",
        "#[serde(rename_all = \"snake_case\")]",
        f"pub enum {rust_type} {{",
    ]
    for item in items:
        lines.append(f"    {pascal(item['name'])} = {item['code']},")
    lines.append("}")
    return lines


def render_rust(spec):
    lines = [
        "// Generated from device_control/protocol/device_control_v1.json. Do not edit.",
        "use serde::{Deserialize, Serialize};",
        "",
        f'pub const CONTRACT_NAME: &str = "{spec["name"]}";',
        f'pub const CONTRACT_VERSION: u8 = {spec["version"]};',
        "",
    ]
    for key, rust_type, _ in ENUMS:
        lines.extend(render_rust_enum(spec[key], rust_type))
        lines.append("")
    return "\n".join(lines)


def render_c_enum(items, c_type, c_prefix):
    lines = ["typedef enum {"]
    for item in items:
        lines.append(
            f"    DENZIC_DEVICE_CONTROL_V1_{c_prefix}_{c_symbol(item['name'])} = {item['code']},"
        )
    lines.append(f"}} denzic_device_control_v1_{c_type}_t;")
    return lines


def render_c(spec):
    lines = [
        "/* Generated from device_control/protocol/device_control_v1.json. Do not edit. */",
        "#ifndef DENZIC_DEVICE_CONTROL_V1_GENERATED_H",
        "#define DENZIC_DEVICE_CONTROL_V1_GENERATED_H",
        "",
        "#include <stdint.h>",
        "",
        "#ifdef __cplusplus",
        "extern \"C\" {",
        "#endif",
        "",
        f'#define DENZIC_DEVICE_CONTROL_V1_CONTRACT_NAME "{spec["name"]}"',
        f'#define DENZIC_DEVICE_CONTROL_V1_CONTRACT_VERSION ({spec["version"]}u)',
        "",
    ]
    for key, _, c_type in ENUMS:
        lines.extend(render_c_enum(spec[key], c_type, c_symbol(c_type)))
        lines.append("")
    lines.extend(["#ifdef __cplusplus", "}", "#endif", "", "#endif", ""])
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
