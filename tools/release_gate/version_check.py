#!/usr/bin/env python3
"""Product-independent release version-consistency gate.

A product repository supplies a JSON config (schema
"denzic.platform.release-version-gate.v1") that names the canonical version
source, the files that must agree with it, and the optional git-tag gate.
Thin product wrappers call this script so their original command-line
interface, output text, and exit-code semantics stay unchanged.

Config fields:
  product        Human-readable product name used in PASS/error text.
  root           Product repository root, relative to the config file.
  canonical      {"path", "kind": "text"|"json", "key"?} canonical version
                 source. "text" reads the whole file trimmed; "json" reads a
                 dotted key (empty segments allowed, e.g. "packages..version").
  semver_error   Error template when the version is not plain semver;
                 "{version}" is substituted.
  must_equal     Sources whose extracted version must equal the canonical
                 one: {"label", "path", "kind": "text"|"json"|"regex",
                 "key"?|"pattern"?}. "regex" takes capture group 1 of a
                 multiline search.
  must_contain   Files that must mention the version (and optionally a
                 reference token such as the canonical file name):
                 {"path", "reference"?}.
  tag_mode       "exists" (git tag --list) or "exact" (git describe
                 --tags --exact-match); enforced only with --require-tag.
  tag_prefix     Tag prefix, defaults to "v".

Exit codes: 0 on PASS, 1 on any gate failure.
"""

import argparse
import json
import re
import subprocess
import sys
from pathlib import Path

SCHEMA = "denzic.platform.release-version-gate.v1"
SEMVER_PATTERN = re.compile(r"^\d+\.\d+\.\d+$")


class GateError(Exception):
    pass


def load_config(path: Path) -> dict:
    with path.open(encoding="utf-8") as handle:
        config = json.load(handle)
    if config.get("schema") != SCHEMA:
        raise GateError(f"Unsupported release-gate config schema: {config.get('schema')}")
    return config


def read_json_key(path: Path, key: str):
    with path.open(encoding="utf-8") as handle:
        data = json.load(handle)
    current = data
    for part in key.split("."):
        if not isinstance(current, dict) or part not in current:
            raise KeyError(key)
        current = current[part]
    return current


def resolve_canonical(root: Path, canonical: dict) -> str:
    path = root / canonical["path"]
    kind = canonical.get("kind", "text")
    if kind == "text":
        if not path.is_file():
            raise GateError(f"{path.name} file is missing.")
        return path.read_text(encoding="utf-8").strip()
    if kind == "json":
        if not path.is_file():
            raise GateError(f"{path.name} file is missing.")
        try:
            value = read_json_key(path, canonical["key"])
        except (ValueError, KeyError) as exc:
            raise GateError(f"{path.name} file is missing.") from exc
        return str(value).strip()
    raise GateError(f"Unsupported canonical source kind: {kind}")


def extract_equal_source(root: Path, source: dict) -> str:
    path = root / source["path"]
    kind = source.get("kind", "text")
    value = None
    try:
        if kind == "json":
            value = read_json_key(path, source["key"])
        elif kind == "regex":
            match = re.search(source["pattern"], path.read_text(encoding="utf-8"), re.MULTILINE)
            value = match.group(1) if match else None
        elif kind == "text":
            value = path.read_text(encoding="utf-8").strip()
        else:
            raise GateError(f"Unsupported version source kind: {kind}")
    except (OSError, ValueError, KeyError):
        value = None
    if value is None:
        raise GateError(f"Missing {source['label']} in {source['path']}")
    return str(value)


def check_must_contain(root: Path, canonical_name: str, version: str, entries) -> None:
    for entry in entries:
        relative = entry["path"]
        path = root / relative
        if not path.is_file():
            raise GateError(f"Required version source file is missing: {relative}")
        content = path.read_text(encoding="utf-8")
        reference = entry.get("reference")
        if reference and reference not in content:
            raise GateError(f"{relative} does not reference the {canonical_name} file.")
        if version not in content:
            raise GateError(f"{relative} does not contain the current fallback version '{version}'.")


def check_tag(root: Path, config: dict, version: str) -> None:
    tag = f"{config.get('tag_prefix', 'v')}{version}"
    mode = config.get("tag_mode")
    if mode == "exists":
        result = subprocess.run(
            ["git", "-C", str(root), "tag", "--list", tag],
            capture_output=True, text=True,
        )
        lines = [line for line in result.stdout.splitlines() if line]
        if result.returncode != 0 or not lines or lines[0] != tag:
            raise GateError(f"Required git tag '{tag}' was not found.")
    elif mode == "exact":
        result = subprocess.run(
            ["git", "describe", "--tags", "--exact-match"],
            cwd=root, capture_output=True, text=True,
        )
        actual = result.stdout.strip()
        if result.returncode != 0 or actual != tag:
            raise GateError(f"Current commit must be tagged {tag}; got {actual or 'no exact tag'}")
    else:
        raise GateError(f"Unsupported tag_mode: {mode}")


def run(config_path: Path, require_tag: bool) -> int:
    config = load_config(config_path)
    root = (config_path.parent / config.get("root", ".")).resolve()
    product = config["product"]
    canonical = config["canonical"]

    version = resolve_canonical(root, canonical)
    if not version:
        raise GateError(f"{Path(canonical['path']).name} file is empty.")

    sources = [(source["label"], extract_equal_source(root, source)) for source in config.get("must_equal", [])]
    if any(value != version for _, value in sources):
        for label, value in sources:
            print(f"{label}: {value}", file=sys.stderr)
        raise GateError(f"{product} version mismatch; expected every source to equal {version}")

    if not SEMVER_PATTERN.match(version):
        raise GateError(config["semver_error"].format(version=version))

    check_must_contain(root, Path(canonical["path"]).name, version, config.get("must_contain", []))

    if require_tag:
        check_tag(root, config, version)

    print(f"PASS: {product} release version is {version}")
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--config", required=True, type=Path, help="Path to the product release-gate config JSON.")
    parser.add_argument("--require-tag", action="store_true", help="Also enforce the configured git-tag gate.")
    args = parser.parse_args()

    try:
        return run(args.config.resolve(), args.require_tag)
    except GateError as exc:
        print(str(exc), file=sys.stderr)
        return 1


if __name__ == "__main__":
    sys.exit(main())
