# XDownload browser extension pack script
# Usage: powershell -ExecutionPolicy Bypass -File build.ps1
# NOTE: keep this file ASCII-only (English) so it parses correctly under
# both Windows PowerShell 5.1 and PowerShell 7 (pwsh) in CI.
$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$out = Join-Path $root "dist"
New-Item -ItemType Directory -Force -Path $out | Out-Null

$zip = Join-Path $out "xdownload-extension-v1.3.0.zip"
if (Test-Path $zip) { Remove-Item $zip }

Compress-Archive -Path `
  (Join-Path $root "manifest.json"), `
  (Join-Path $root "content.js"), `
  (Join-Path $root "background.js"), `
  (Join-Path $root "popup.html"), `
  (Join-Path $root "popup.js"), `
  (Join-Path $root "icons") `
  -DestinationPath $zip

Write-Host "Extension zip ready: $zip"
