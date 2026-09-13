# Contributing

Denzic Platform exists to keep host and embedded products on the same portable
contracts. Contributions are welcome when they preserve that boundary.

## Before changing a contract

Open an issue for a wire-format or public-behavior change. Describe the product
need, compatibility impact, rollout order, and how older hosts or devices should
behave. A contract change should update its JSON definition, generated source,
host and embedded implementations, and tests in one pull request.

Add an active capability to `capabilities.json` only after its protocol, host,
and embedded layers exist. Keep hardware, operating-system, storage, transport,
and UI side effects in product adapters.

## Validation

Run the repository gate before opening a pull request:

```powershell
pwsh -NoProfile -File .\tools\verify.ps1
```

The gate checks generated source, adapter compatibility, Rust formatting and
tests, TypeScript protocol tests, and portable C builds and tests.

## Pull requests

- Keep each change focused on one capability or one cross-cutting correction.
- Explain the compatibility and migration behavior when a contract changes.
- Add tests at the shared decision boundary, including rejection and recovery
  paths where they matter.
- Update product adapters in their own repositories when integration changes.
- Do not commit credentials, recordings, transcripts, device logs, local build
  output, generated diagnostic bundles, or release artifacts.
