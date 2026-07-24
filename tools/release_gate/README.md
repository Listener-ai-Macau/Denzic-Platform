# Release Gate

Product-independent release-gate tooling. Product repositories keep thin
wrappers (`.ps1` / `.mjs`) that preserve their original command-line
interface, output text, and exit codes, and delegate the shared logic here
with a per-product JSON config.

## `version_check.py`

Release version-consistency gate: reads the canonical version from the
product's version source, requires every configured source to agree,
enforces plain semver, optionally requires a git tag (`--require-tag`).

```
python tools/release_gate/version_check.py --config <product-config.json> [--require-tag]
```

Config schema `denzic.platform.release-version-gate.v1`:

| field | meaning |
| --- | --- |
| `product` | product name used in PASS/error text |
| `root` | product repo root, relative to the config file |
| `canonical` | `{"path", "kind": "text"\|"json", "key"?}` canonical version source |
| `semver_error` | error template for non-semver versions; `{version}` substituted |
| `must_equal` | sources that must equal the canonical version (`json` dotted key or `regex` capture group 1) |
| `must_contain` | files that must mention the version, plus an optional `reference` token |
| `tag_mode` | `"exists"` (`git tag --list`) or `"exact"` (`git describe --tags --exact-match`) |
| `tag_prefix` | tag prefix, default `"v"` |

Consumers: Listener-Firmware (`tools/check_release_version.ps1` with
`tools/release_version_gate.json`) and Listener-Type
(`scripts/check-release-version.mjs` with `scripts/release-version-gate.json`).

## `self_test.py`

Fixture-based self-test wired into `tools/verify.ps1`:

```
python tools/release_gate/self_test.py
```
