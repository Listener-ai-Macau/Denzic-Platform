#!/usr/bin/env python3
"""Self-test for the release-gate version-consistency tool.

Builds firmware-style and host-app-style fixture repositories in a temp
directory and asserts exit codes plus the exact PASS/error text that product
wrappers depend on. Run: python tools/release_gate/self_test.py
"""

import json
import subprocess
import sys
import tempfile
from pathlib import Path

TOOL = Path(__file__).resolve().parent / "version_check.py"

FAILURES = []


def run_gate(config_path: Path, *extra: str):
    result = subprocess.run(
        [sys.executable, str(TOOL), "--config", str(config_path), *extra],
        capture_output=True, text=True,
    )
    return result.returncode, result.stdout, result.stderr


def check(name: str, condition: bool, detail: str = "") -> None:
    if condition:
        print(f"PASS: {name}")
    else:
        FAILURES.append(name)
        print(f"FAIL: {name} {detail}")


def write_json(path: Path, payload) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(payload, indent=2), encoding="utf-8")


def write_text(path: Path, text: str) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(text, encoding="utf-8")


def git(repo: Path, *args: str) -> subprocess.CompletedProcess:
    return subprocess.run(["git", "-C", str(repo), *args], capture_output=True, text=True)


def firmware_config() -> dict:
    return {
        "schema": "denzic.platform.release-version-gate.v1",
        "product": "Listener Firmware",
        "root": ".",
        "canonical": {"path": "VERSION", "kind": "text"},
        "semver_error": "Release VERSION must be plain semver like 1.0.0; got '{version}'.",
        "must_contain": [
            {"path": "CMakeLists.txt", "reference": "VERSION"},
            {"path": "tools\\package_ota_firmware.ps1", "reference": "VERSION"},
        ],
        "tag_mode": "exists",
    }


def type_config() -> dict:
    return {
        "schema": "denzic.platform.release-version-gate.v1",
        "product": "Listener Type",
        "root": ".",
        "canonical": {"path": "package.json", "kind": "json", "key": "version"},
        "semver_error": "Listener Type release version must be plain semver, got {version}",
        "must_equal": [
            {"label": "package.json", "path": "package.json", "kind": "json", "key": "version"},
            {"label": "package-lock root package", "path": "package-lock.json", "kind": "json", "key": "packages..version"},
            {"label": "src-tauri/Cargo.toml", "path": "src-tauri/Cargo.toml", "kind": "regex",
             "pattern": "^version\\s*=\\s*\"([^\"]+)\""},
        ],
        "tag_mode": "exact",
    }


def build_firmware_repo(root: Path, version: str = "1.2.3", referenced: bool = True) -> Path:
    write_text(root / "VERSION", version + "\n")
    mention = f'project(demo VERSION {version})' if referenced else "project(demo)"
    write_text(root / "CMakeLists.txt", f"cmake_minimum_required(VERSION 3.16)\n{mention}\n")
    write_text(root / "tools" / "package_ota_firmware.ps1", f"# VERSION fallback {version}\n")
    config_path = root / "release_version_gate.json"
    write_json(config_path, firmware_config())
    return config_path


def build_type_repo(root: Path, cargo_version: str = "2.0.0") -> Path:
    write_json(root / "package.json", {"name": "listener-type", "version": "2.0.0"})
    write_json(root / "package-lock.json", {"version": "2.0.0", "packages": {"": {"version": "2.0.0"}}})
    write_text(root / "src-tauri" / "Cargo.toml", f'[package]\nname = "listener-type"\nversion = "{cargo_version}"\n')
    config_path = root / "release-version-gate.json"
    write_json(config_path, type_config())
    return config_path


def main() -> int:
    temp_root = Path(tempfile.mkdtemp(prefix="release-gate-selftest-"))

    # Firmware-style gate: pass and each failure mode.
    config = build_firmware_repo(temp_root / "fw-ok")
    rc, out, _ = run_gate(config)
    check("firmware-style pass", rc == 0 and out.strip() == "PASS: Listener Firmware release version is 1.2.3", out)

    config = build_firmware_repo(temp_root / "fw-missing")
    (temp_root / "fw-missing" / "VERSION").unlink()
    rc, _, err = run_gate(config)
    check("firmware missing VERSION", rc == 1 and "VERSION file is missing." in err, err)

    config = build_firmware_repo(temp_root / "fw-badsemver", version="1.2.3-beta")
    rc, _, err = run_gate(config)
    check("firmware non-semver rejected", rc == 1 and "must be plain semver like 1.0.0; got '1.2.3-beta'." in err, err)

    config = build_firmware_repo(temp_root / "fw-stale", referenced=False)
    rc, _, err = run_gate(config)
    check("firmware stale fallback version", rc == 1 and "does not contain the current fallback version '1.2.3'." in err, err)

    config = build_firmware_repo(temp_root / "fw-missing-source")
    (temp_root / "fw-missing-source" / "CMakeLists.txt").unlink()
    rc, _, err = run_gate(config)
    check("firmware missing source file", rc == 1 and "Required version source file is missing: CMakeLists.txt" in err, err)

    # Host-app-style gate: pass, mismatch, and extraction failure.
    config = build_type_repo(temp_root / "type-ok")
    rc, out, _ = run_gate(config)
    check("host-app-style pass", rc == 0 and out.strip() == "PASS: Listener Type release version is 2.0.0", out)

    config = build_type_repo(temp_root / "type-mismatch", cargo_version="2.0.1")
    rc, _, err = run_gate(config)
    check(
        "host-app mismatch rejected",
        rc == 1
        and "src-tauri/Cargo.toml: 2.0.1" in err
        and "Listener Type version mismatch; expected every source to equal 2.0.0" in err,
        err,
    )

    config = build_type_repo(temp_root / "type-missing-source")
    (temp_root / "type-missing-source" / "src-tauri" / "Cargo.toml").unlink()
    rc, _, err = run_gate(config)
    check("host-app missing source", rc == 1 and "Missing src-tauri/Cargo.toml in src-tauri/Cargo.toml" in err, err)

    # Git tag gates.
    tag_repo = temp_root / "fw-tag"
    config = build_firmware_repo(tag_repo)
    git(tag_repo, "init")
    git(tag_repo, "add", ".")
    git(tag_repo, "-c", "user.name=release-gate-test", "-c", "user.email=release-gate-test@example.invalid",
        "commit", "-m", "init")
    git(tag_repo, "tag", "v1.2.3")
    rc, out, _ = run_gate(config, "--require-tag")
    check("tag exists gate pass", rc == 0 and "PASS: Listener Firmware release version is 1.2.3" in out, out)

    untagged_repo = temp_root / "fw-untagged"
    config = build_firmware_repo(untagged_repo)
    git(untagged_repo, "init")
    git(untagged_repo, "add", ".")
    git(untagged_repo, "-c", "user.name=release-gate-test", "-c", "user.email=release-gate-test@example.invalid",
        "commit", "-m", "init")
    rc, _, err = run_gate(config, "--require-tag")
    check("tag exists gate reject", rc == 1 and "Required git tag 'v1.2.3' was not found." in err, err)

    type_tag_repo = temp_root / "type-tag"
    config = build_type_repo(type_tag_repo)
    git(type_tag_repo, "init")
    git(type_tag_repo, "add", ".")
    git(type_tag_repo, "-c", "user.name=release-gate-test", "-c", "user.email=release-gate-test@example.invalid",
        "commit", "-m", "init")
    git(type_tag_repo, "tag", "v2.0.0")
    rc, out, _ = run_gate(config, "--require-tag")
    check("tag exact gate pass", rc == 0 and "PASS: Listener Type release version is 2.0.0" in out, out)

    write_text(type_tag_repo / "note.txt", "move HEAD past the tag\n")
    git(type_tag_repo, "add", ".")
    git(type_tag_repo, "-c", "user.name=release-gate-test", "-c", "user.email=release-gate-test@example.invalid",
        "commit", "-m", "past tag")
    rc, _, err = run_gate(config, "--require-tag")
    check("tag exact gate reject", rc == 1 and "Current commit must be tagged v2.0.0; got no exact tag" in err, err)

    if FAILURES:
        print(f"FAIL: release gate self-test failed ({len(FAILURES)} case(s)).")
        return 1
    print("PASS: release gate self-test completed.")
    return 0


if __name__ == "__main__":
    sys.exit(main())
