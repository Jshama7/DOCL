# DOCL portable build.
# Usage: powershell -ExecutionPolicy Bypass -File scripts/build-portable.ps1
# Output: dist-portable/DOCL_<version>_portable.zip (DOCL.exe + portable.txt + empty data/)

$ErrorActionPreference = "Stop"

$repoRoot = Split-Path -Parent $PSScriptRoot

Push-Location $repoRoot
try {
    pnpm tauri build --no-bundle
    if ($LASTEXITCODE -ne 0) {
        throw "Build failed with exit code $LASTEXITCODE"
    }
} finally {
    Pop-Location
}

$exe = Join-Path $repoRoot "src-tauri\target\release\mc-launcher.exe"
if (-not (Test-Path $exe)) {
    throw "Release exe not found: $exe"
}

$conf = Get-Content (Join-Path $repoRoot "src-tauri\tauri.conf.json") -Raw | ConvertFrom-Json
$version = $conf.version

$outDir = Join-Path $repoRoot "dist-portable"
$pkgDir = Join-Path $outDir "DOCL"
if (Test-Path $pkgDir) {
    Remove-Item $pkgDir -Recurse -Force
}
New-Item -ItemType Directory -Path $pkgDir | Out-Null

Copy-Item $exe (Join-Path $pkgDir "DOCL.exe")

# Marker file: its presence switches the launcher to portable mode,
# so config / accounts / skins / default .minecraft live in .\data.
$notes = @(
    "DOCL portable build",
    "",
    "Config, accounts, skins and the default .minecraft folder live in .\data.",
    "Delete the whole folder to uninstall. Keep this file so the launcher stays portable."
)
Set-Content -Path (Join-Path $pkgDir "portable.txt") -Value $notes -Encoding UTF8

$zip = Join-Path $outDir ("DOCL_{0}_portable.zip" -f $version)
if (Test-Path $zip) {
    Remove-Item $zip -Force
}
Compress-Archive -Path (Join-Path $pkgDir "*") -DestinationPath $zip -CompressionLevel Optimal

$size = (Get-Item $zip).Length / 1MB
$hash = (Get-FileHash $zip -Algorithm SHA256).Hash
Write-Host ("portable package: {0}" -f $zip) -ForegroundColor Green
Write-Host ("size: {0:N1} MB" -f $size)
Write-Host ("sha256: {0}" -f $hash)
