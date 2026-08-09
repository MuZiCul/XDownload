# XDownload 浏览器扩展打包脚本
# 用法: powershell -ExecutionPolicy Bypass -File build.ps1
$ErrorActionPreference = "Stop"

$root = Split-Path -Parent $MyInvocation.MyCommand.Path
$out = Join-Path $root "dist"
New-Item -ItemType Directory -Force -Path $out | Out-Null

$zip = Join-Path $out "xdownload-extension-v1.0.0.zip"
if (Test-Path $zip) { Remove-Item $zip }

Compress-Archive -Path `
  (Join-Path $root "manifest.json"), `
  (Join-Path $root "content.js"), `
  (Join-Path $root "icons") `
  -DestinationPath $zip

Write-Host "打包完成: $zip"
