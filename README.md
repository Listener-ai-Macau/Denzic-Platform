# Denzic Platform

Denzic Platform is the shared protocol and state-machine layer behind Denzic
desktop apps and devices. It keeps both sides of a feature on the same contract
without pulling ESP-IDF, Windows APIs, Bluetooth stacks, storage drivers, or UI
code into the shared library.

[Listener Type](https://github.com/Listener-ai-Macau/Listener-Type) and
[Listener Firmware](https://github.com/Listener-ai-Macau/Listener-Firmware)
pin this repository as a Git submodule. That gives each product a reviewed,
repeatable platform revision instead of silently following the newest commit.

## What lives here

The repository contains portable C cores for embedded products, Rust host
cores, TypeScript OTA support, versioned JSON wire contracts, and generators
that keep those implementations aligned.

The current contracts cover:

- audio packets, lossless coding, flow control, and host capture
- device discovery, capabilities, settings, and command transactions
- BLE pairing policy and Windows BLE behavior
- battery, power, startup health, and safe-mode decisions
- diagnostics and cross-device observability
- wake decisions and speaker verification
- OTA manifests, transfer state, verification, and rollback decisions
- release checks shared by product repositories

`capabilities.json` is the source of truth for what is active. Every active
capability must name its protocol, host implementation, and embedded
implementation; CI rejects incomplete entries and generated files that have
drifted from their contract.

## Design boundary

Platform code decides portable behavior. Product adapters perform side effects.
For example, a shared power policy can decide that shutdown is allowed, while a
firmware adapter owns the GPIO, scheduler, LED, and hardware shutdown calls.
The same boundary keeps microphones, file placement, BLE stacks, flash storage,
and operating-system APIs in the product that actually owns them.

Most capabilities follow this shape:

```text
<capability>/protocol/       versioned contract
<capability>/host/           Rust or TypeScript host core
<capability>/embedded/c/     portable embedded C core
tools/generate_*.py          generated-source checks
```

## Validate a change

On Windows with PowerShell 7, Python, Node.js, Rust, CMake, and Visual Studio
Build Tools installed:

```powershell
pwsh -NoProfile -File .\tools\verify.ps1
```

This validates the capability inventory, regenerates contracts in check mode,
checks product adapter compatibility, runs the Rust and TypeScript tests, and
builds and tests the portable C cores. The same command runs in GitHub Actions.

When using this repository through a product checkout, initialize the pinned
revision with:

```bash
git submodule update --init --recursive
```

## Contributing

Start with [CONTRIBUTING.md](CONTRIBUTING.md). Use GitHub Issues for defects and
proposals that affect a shared contract. Product UI, hardware wiring, and
device-specific behavior belong in the relevant product repository.

Security issues should be reported privately as described in
[SECURITY.md](SECURITY.md). General help and repository boundaries are covered
in [SUPPORT.md](SUPPORT.md).

Denzic Platform is available under the [Apache License 2.0](LICENSE).
