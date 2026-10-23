# DOCL release build.
# Remaps compile-time paths so the shipped exe does not contain
# C:\Users\<username>\.cargo\... strings from dependency panic locations.
# Usage: powershell -ExecutionPolicy Bypass -File scripts/build-release.ps1

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot
$userProfilePath = $env:USERPROFILE
if (-not $userProfilePath) {
    throw "USERPROFILE is not set, cannot build path remapping flags"
}

$remapTarget = "C:\build"
$env:RUSTFLAGS = "--remap-path-prefix=$userProfilePath=$remapTarget"

Write-Host "RUSTFLAGS=$env:RUSTFLAGS" -ForegroundColor Cyan

Push-Location $repoRoot
try {
    pnpm tauri build
    if ($LASTEXITCODE -ne 0) {
        throw "Build failed with exit code $LASTEXITCODE"
    }
} finally {
    Pop-Location
}

# Self check: the user name must be gone from the binary.
$exe = Join-Path $repoRoot "src-tauri\target\release\mc-launcher.exe"
if (Test-Path $exe) {
    $text = [System.Text.Encoding]::ASCII.GetString([System.IO.File]::ReadAllBytes($exe))
    $leaf = Split-Path $userProfilePath -Leaf
    if ($text.Contains($leaf)) {
        Write-Warning "Binary still contains user name: $leaf"
    } else {
        Write-Host "Self check passed: user name '$leaf' not found in exe" -ForegroundColor Green
    }
}
