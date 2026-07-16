#!/usr/bin/env python3
"""Resolve the Platform contract tuple and reject incompatible adapter declarations."""

import argparse
import json
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]


def load_json(path: Path):
    with path.open(encoding="utf-8") as handle:
        return json.load(handle)


def resolve_contracts():
    inventory = load_json(ROOT / "capabilities.json")
    if inventory.get("schema") != "denzic.platform.capabilities.v1":
        raise ValueError("unsupported capabilities inventory schema")

    resolved = {}
    for capability in inventory.get("capabilities", []):
        if capability.get("status") != "active":
            continue
        protocol_paths = capability.get("layers", {}).get("protocol", [])
        if len(protocol_paths) != 1:
            raise ValueError(f"{capability.get('id')} must declare exactly one protocol source")
        spec = load_json(ROOT / protocol_paths[0])
        contract = capability.get("contract")
        if spec.get("name") != contract or not isinstance(spec.get("version"), int):
            raise ValueError(f"{capability.get('id')} contract identity does not match its protocol source")
        resolved[capability["id"]] = {"contract": contract, "version": spec["version"]}

    observability = resolved.get("observability")
    if observability is None:
        raise ValueError("observability contract is required to identify the BLE lifecycle")
    return {
        "ble": observability,
        "audio": resolved["audio"],
        "ota": resolved["ota"],
        "observability": observability,
    }


def compatibility_errors(declared, resolved):
    if not isinstance(declared, dict):
        return ["adapter contracts must be an object"]
    errors = []
    for capability, expected in resolved.items():
        actual = declared.get(capability)
        if actual != expected:
            errors.append(f"{capability}: expected {expected}, got {actual}")
    extras = sorted(set(declared) - set(resolved))
    if extras:
        errors.append(f"unsupported capabilities: {', '.join(extras)}")
    return errors


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--manifest", required=True, type=Path)
    parser.add_argument("--expect-rejected", action="store_true")
    args = parser.parse_args()

    manifest = load_json(args.manifest)
    resolved = resolve_contracts()
    errors = compatibility_errors(manifest.get("contracts"), resolved)
    accepted = not errors
    result = {
        "adapter": manifest.get("adapter"),
        "accepted": accepted,
        "contracts": resolved,
        "errors": errors,
    }
    print(json.dumps(result, sort_keys=True))
    if accepted == args.expect_rejected:
        raise SystemExit("adapter compatibility result did not match expectation")


if __name__ == "__main__":
    main()
