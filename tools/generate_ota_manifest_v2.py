#!/usr/bin/env python3
import argparse
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SPEC_PATH = ROOT / "ota" / "protocol" / "ota_manifest_v2.json"
RUST_PATH = ROOT / "ota" / "host" / "rust" / "src" / "generated_manifest.rs"


def render_rust(spec):
    channels = ", ".join(f'"{channel}"' for channel in spec["channels"])
    rollback_methods = ", ".join(f'"{method}"' for method in spec["rollback_methods"])
    lines = [
        "// Generated from ota/protocol/ota_manifest_v2.json. Do not edit.",
        f'pub const MANIFEST_NAME: &str = "{spec["name"]}";',
        f'pub const MANIFEST_SCHEMA_VERSION: u64 = {spec["schema_version"]};',
        f'pub const MANIFEST_FILE_NAME: &str = "{spec["manifest_file_name"]}";',
        f'pub const MANIFEST_PACKAGE_FILE_NAME: &str = "{spec["package_file_name"]}";',
        f'pub const MANIFEST_FIRMWARE_VERSION_MAX_CHARS: usize = {spec["firmware_version_max_chars"]};',
        f'pub const MANIFEST_SHA256_HEX_CHARS: usize = {spec["sha256_hex_chars"]};',
        f'pub const MANIFEST_DEFAULT_GATT_CHUNK_BYTES: u64 = {spec["default_gatt_chunk_bytes"]};',
        f"pub const MANIFEST_CHANNELS: [&str; {len(spec['channels'])}] = [{channels}];",
        f"pub const MANIFEST_ROLLBACK_METHODS: [&str; {len(spec['rollback_methods'])}] = [{rollback_methods}];",
    ]
    return "\n".join(lines) + "\n"


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
    if spec["schema_version"] != 2:
        raise SystemExit("ota_manifest_v2.json must pin schema_version 2 (schema_version 1 is retired)")
    for section, fields in spec["required_fields"].items():
        if not section.strip():
            raise SystemExit("required_fields section names must be non-empty")
        for kind, names in fields.items():
            if kind not in ("string", "bool", "positive_integer"):
                raise SystemExit(f"required_fields.{section} uses unknown field kind: {kind}")
            for name in names:
                if not name.strip():
                    raise SystemExit(f"required_fields.{section}.{kind} entries must be non-empty")
    write_or_check(RUST_PATH, render_rust(spec), args.check)


if __name__ == "__main__":
    main()
