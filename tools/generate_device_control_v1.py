#!/usr/bin/env python3
"""Generate the shared device-control v1 enum surface from its contract."""

import argparse
import json
import re
import uuid
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

    settings = spec.get("settings_revision")
    if not isinstance(settings, dict):
        raise SystemExit("settings_revision must be an object")
    try:
        uuid.UUID(settings.get("characteristic_uuid", ""))
    except (ValueError, AttributeError, TypeError) as error:
        raise SystemExit(
            f"settings_revision.characteristic_uuid must be a canonical UUID: {error}"
        ) from error
    for key in ("value_schema", "value_field"):
        value = settings.get(key)
        if not isinstance(value, str) or not re.fullmatch(r"[a-z][a-z0-9_.]*", value):
            raise SystemExit(f"settings_revision.{key} has invalid value: {value!r}")

    ec11 = spec.get("ec11_recovery")
    if not isinstance(ec11, dict):
        raise SystemExit("ec11_recovery must be an object")
    for key in EC11_MESSAGES:
        value = ec11.get(key)
        if not isinstance(value, str) or not value:
            raise SystemExit(f"ec11_recovery.{key} must be a non-empty string")
        if not re.fullmatch(r"[ -~]+", value) or "\\" in value or '"' in value:
            raise SystemExit(
                f"ec11_recovery.{key} must be printable ASCII without quotes: {value!r}"
            )


EC11_MESSAGES = ("notice", "prepare_notice", "ack", "prepare_ack")


def c_uuid_bytes(value):
    return ", ".join(f"0x{byte:02x}" for byte in reversed(uuid.UUID(value).bytes))


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
    lines.extend(render_rust_wire_constants(spec))
    return "\n".join(lines)


def render_rust_wire_constants(spec):
    settings = spec["settings_revision"]
    settings_uuid = uuid.UUID(settings["characteristic_uuid"])
    lines = [
        f'pub const SETTINGS_REVISION_CHARACTERISTIC_UUID: &str = "{settings_uuid}";',
        f"pub const SETTINGS_REVISION_CHARACTERISTIC_UUID_U128: u128 = 0x{settings_uuid.int:032x};",
        f'pub const SETTINGS_REVISION_VALUE_SCHEMA: &str = "{settings["value_schema"]}";',
        f'pub const SETTINGS_REVISION_VALUE_FIELD: &str = "{settings["value_field"]}";',
        "",
        "// EC11 recovery handshake tokens. Notices are notified by the device;",
        "// acknowledgements are written by the host with a trailing line feed,",
        "// which the device strips before matching the token.",
    ]
    for key in EC11_MESSAGES:
        symbol = f"EC11_RECOVERY_{c_symbol(key)}"
        token = spec["ec11_recovery"][key]
        lines.append(f'pub const {symbol}: &[u8] = b"{token}";')
        if key.endswith("ack"):
            lines.append(f'pub const {symbol}_WRITE: &[u8] = b"{token}\\n";')
    lines.append("")
    return lines


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
    lines.extend(render_c_wire_constants(spec))
    lines.extend(["#ifdef __cplusplus", "}", "#endif", "", "#endif", ""])
    return "\n".join(lines)


def render_c_wire_constants(spec):
    settings = spec["settings_revision"]
    lines = [
        f'#define DENZIC_DEVICE_CONTROL_V1_SETTINGS_REVISION_UUID_TEXT "{settings["characteristic_uuid"]}"',
        f"#define DENZIC_DEVICE_CONTROL_V1_SETTINGS_REVISION_UUID_BYTES {c_uuid_bytes(settings['characteristic_uuid'])}",
        f'#define DENZIC_DEVICE_CONTROL_V1_SETTINGS_REVISION_VALUE_SCHEMA "{settings["value_schema"]}"',
        f'#define DENZIC_DEVICE_CONTROL_V1_SETTINGS_REVISION_VALUE_FIELD "{settings["value_field"]}"',
        "",
        "/* EC11 recovery handshake tokens; host acknowledgements are line-feed terminated. */",
    ]
    for key in EC11_MESSAGES:
        symbol = f"DENZIC_DEVICE_CONTROL_V1_EC11_RECOVERY_{c_symbol(key)}"
        lines.append(f'#define {symbol} "{spec["ec11_recovery"][key]}"')
    lines.append("")
    return lines


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
