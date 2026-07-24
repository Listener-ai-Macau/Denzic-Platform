"""Shared generator for compact enum/constant platform contracts."""

import argparse
import json
import re
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def pascal(value):
    return "".join(part[:1].upper() + part[1:] for part in value.split("_"))


def symbol(value):
    return re.sub(r"[^A-Za-z0-9]+", "_", value).upper()


def load_spec(capability):
    path = ROOT / capability / "protocol" / f"{capability}_v1.json"
    spec = json.loads(path.read_text(encoding="utf-8"))
    if spec.get("name") != f"denzic_{capability}_v1" or spec.get("version") != 1:
        raise SystemExit(f"{capability} v1 name/version are fixed")
    enums = spec.get("enums")
    if not isinstance(enums, dict) or not enums:
        raise SystemExit("enums must be a non-empty object")
    for enum_name, values in enums.items():
        if not re.fullmatch(r"[a-z][a-z0-9_]*", enum_name):
            raise SystemExit(f"invalid enum name: {enum_name!r}")
        if not isinstance(values, list) or not values:
            raise SystemExit(f"{enum_name} must be a non-empty array")
        names = set()
        codes = set()
        for item in values:
            name = item.get("name") if isinstance(item, dict) else None
            code = item.get("code") if isinstance(item, dict) else None
            if not isinstance(name, str) or not re.fullmatch(r"[a-z][a-z0-9_]*", name):
                raise SystemExit(f"{enum_name} has invalid name: {name!r}")
            if not isinstance(code, int) or not 0 <= code <= 255:
                raise SystemExit(f"{enum_name}.{name} must have a u8 code")
            if name in names or code in codes:
                raise SystemExit(f"{enum_name} contains a duplicate")
            names.add(name)
            codes.add(code)
    constants = spec.get("constants", {})
    if not isinstance(constants, dict):
        raise SystemExit("constants must be an object")
    for name, value in constants.items():
        if not re.fullmatch(r"[a-z][a-z0-9_]*", name):
            raise SystemExit(f"invalid constant name: {name!r}")
        if not isinstance(value, int) or not 0 <= value <= 0xFFFFFFFF:
            raise SystemExit(f"{name} must be a u32")
    return spec


def render_rust(capability, spec):
    lines = [
        f"// Generated from {capability}/protocol/{capability}_v1.json. Do not edit.",
        "use serde::{Deserialize, Serialize};",
        "",
        f'pub const CONTRACT_NAME: &str = "{spec["name"]}";',
        f'pub const CONTRACT_VERSION: u8 = {spec["version"]};',
        "",
    ]
    for enum_name, values in spec["enums"].items():
        rust_name = pascal(enum_name)
        lines.extend(
            [
                "#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]",
                "#[repr(u8)]",
                "#[serde(rename_all = \"snake_case\")]",
                f"pub enum {rust_name} {{",
            ]
        )
        for item in values:
            lines.append(f"    {pascal(item['name'])} = {item['code']},")
        lines.extend(["}", ""])
    for name, value in spec.get("constants", {}).items():
        lines.append(f"pub const {symbol(name)}: u32 = {value};")
    lines.append("")
    return "\n".join(lines)


def render_c(capability, spec):
    prefix = f"DENZIC_{symbol(capability)}_V1"
    guard = f"{prefix}_GENERATED_H"
    lines = [
        f"/* Generated from {capability}/protocol/{capability}_v1.json. Do not edit. */",
        f"#ifndef {guard}",
        f"#define {guard}",
        "",
        "#include <stdint.h>",
        "",
        f'#define {prefix}_CONTRACT_NAME "{spec["name"]}"',
        f"#define {prefix}_CONTRACT_VERSION ({spec['version']}u)",
        "",
    ]
    for enum_name, values in spec["enums"].items():
        enum_symbol = symbol(enum_name)
        lines.append("typedef enum {")
        for item in values:
            lines.append(
                f"    {prefix}_{enum_symbol}_{symbol(item['name'])} = {item['code']},"
            )
        lines.extend(
            [
                f"}} denzic_{capability}_v1_{enum_name}_t;",
                "",
            ]
        )
    for name, value in spec.get("constants", {}).items():
        lines.append(f"#define {prefix}_{symbol(name)} ({value}u)")
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


def generate(capability):
    parser = argparse.ArgumentParser()
    parser.add_argument("--check", action="store_true")
    args = parser.parse_args()
    spec = load_spec(capability)
    rust_path = ROOT / capability / "host" / "rust" / "src" / "generated.rs"
    c_path = (
        ROOT
        / capability
        / "embedded"
        / "c"
        / "include"
        / f"denzic_{capability}_v1_generated.h"
    )
    write_or_check(rust_path, render_rust(capability, spec), args.check)
    write_or_check(c_path, render_c(capability, spec), args.check)
