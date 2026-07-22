[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path

if ($PSVersionTable.PSVersion.Major -lt 7) {
    throw "PowerShell 7 or newer is required. Run with pwsh -NoProfile -File."
}

Push-Location $repoRoot
try {
    $inventory = Get-Content -LiteralPath .\capabilities.json -Raw -Encoding UTF8 | ConvertFrom-Json
    if ($inventory.schema -ne "denzic.platform.capabilities.v1") {
        throw "Unsupported capabilities.json schema: $($inventory.schema)"
    }
    foreach ($capability in @($inventory.capabilities | Where-Object status -eq "active")) {
        foreach ($layer in @($inventory.required_layers)) {
            $paths = @($capability.layers.$layer)
            if ($paths.Count -eq 0) {
                throw "Active capability '$($capability.id)' is missing the '$layer' layer."
            }
            foreach ($path in $paths) {
                if (-not (Test-Path -LiteralPath (Join-Path $repoRoot $path) -PathType Leaf)) {
                    throw "Active capability '$($capability.id)' has missing $layer path: $path"
                }
            }
        }
    }

    python .\tools\generate_ota_v1.py --check
    if ($LASTEXITCODE -ne 0) { throw "OTA generated-source check failed." }

    python .\tools\generate_audio_v1.py --check
    if ($LASTEXITCODE -ne 0) { throw "Audio generated-source check failed." }

    python .\tools\generate_ble_windows_v1.py --check
    if ($LASTEXITCODE -ne 0) { throw "BLE Windows generated-source check failed." }

    python .\tools\generate_device_control_v1.py --check
    if ($LASTEXITCODE -ne 0) { throw "Device-control generated-source check failed." }

    python .\tools\generate_observability_v1.py --check
    if ($LASTEXITCODE -ne 0) { throw "Observability generated-source check failed." }

    python .\tools\verify_adapter_compatibility.py --manifest .\compatibility\listener_adapter_v1.json
    if ($LASTEXITCODE -ne 0) { throw "Supported adapter compatibility check failed." }

    python .\tools\verify_adapter_compatibility.py --manifest .\compatibility\listener_adapter_unsupported_ota.json --expect-rejected
    if ($LASTEXITCODE -ne 0) { throw "Unsupported adapter compatibility check did not reject the declaration." }

    python .\tools\verify_observability_adapter_docs.py
    if ($LASTEXITCODE -ne 0) { throw "Observability adapter documentation check failed." }

    cargo fmt --all -- --check
    if ($LASTEXITCODE -ne 0) { throw "Rust formatting check failed." }

    cargo test --workspace
    if ($LASTEXITCODE -ne 0) { throw "Rust workspace tests failed." }

    node --experimental-strip-types .\ota\host\typescript\tests\protocol.test.ts
    if ($LASTEXITCODE -ne 0) { throw "TypeScript host-core tests failed." }

    cmake -S . -B .\build\msvc -G "Visual Studio 17 2022" -A x64
    if ($LASTEXITCODE -ne 0) { throw "CMake configure failed." }
    cmake --build .\build\msvc --config Release
    if ($LASTEXITCODE -ne 0) { throw "C firmware-core build failed." }
    ctest --test-dir .\build\msvc -C Release --output-on-failure
    if ($LASTEXITCODE -ne 0) { throw "C firmware-core tests failed." }
} finally {
    Pop-Location
}
