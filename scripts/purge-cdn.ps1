# Release cleanup: purge jsdelivr CDN cache for update.json and verify the live version.
# Usage: powershell -ExecutionPolicy Bypass -File scripts/purge-cdn.ps1 [-Version 1.5.1]
param(
  [string]$Version = ""
)

$ErrorActionPreference = "Continue"
$path = "gh/dawo4/DOCL@main/update.json"
$proxy = $env:HTTPS_PROXY
if (-not $proxy) { $proxy = $env:HTTP_PROXY }

function Invoke-Purge([string]$label) {
  try {
    if ($proxy) {
      Invoke-RestMethod "https://purge.jsdelivr.net/$path" -Proxy $proxy -TimeoutSec 20 | Out-Null
    } else {
      Invoke-RestMethod "https://purge.jsdelivr.net/$path" -TimeoutSec 20 | Out-Null
    }
    Write-Host "purging via jsdelivr..." -ForegroundColor Cyan
    return $true
  } catch {
    Write-Warning "jsdelivr purge attempt failed: $_"
    return $false
  }
}

$purged = Invoke-Purge
Start-Sleep -Seconds 3

$live = ""
try {
  if ($proxy) {
    $live = (Invoke-RestMethod "https://cdn.jsdelivr.net/$path" -Proxy $proxy -TimeoutSec 20).version
  } else {
    $live = (Invoke-RestMethod "https://cdn.jsdelivr.net/$path" -TimeoutSec 20).version
  }
  Write-Host "CDN live version: $live" -ForegroundColor Green
} catch {
  Write-Warning "failed to read CDN version: $_"
}

if ($Version -and $live -ne $Version) {
  Write-Warning "CDN still serves $live, expected $Version - run this script again in a minute"
  exit 1
}
