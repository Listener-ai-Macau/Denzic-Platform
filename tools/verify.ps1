[CmdletBinding()]
param()

$ErrorActionPreference = "Stop"
$repoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path

Push-Location $repoRoot
try {
    python .\tools\generate_ota_v1.py --check
    if ($LASTEXITCODE -ne 0) { throw "OTA generated-source check failed." }

    cargo fmt --all -- --check
    if ($LASTEXITCODE -ne 0) { throw "Rust formatting check failed." }

    cargo test --workspace
    if ($LASTEXITCODE -ne 0) { throw "Rust workspace tests failed." }

    cmake -S . -B .\build\msvc -G "Visual Studio 17 2022" -A x64
    if ($LASTEXITCODE -ne 0) { throw "CMake configure failed." }
    cmake --build .\build\msvc --config Release
    if ($LASTEXITCODE -ne 0) { throw "C firmware-core build failed." }
    ctest --test-dir .\build\msvc -C Release --output-on-failure
    if ($LASTEXITCODE -ne 0) { throw "C firmware-core tests failed." }
} finally {
    Pop-Location
}
