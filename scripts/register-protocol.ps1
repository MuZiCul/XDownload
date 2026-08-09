# 手动注册/反注册 xdownload:// 自定义协议（Windows 当前用户）
# 用法:
#   注册:   powershell -ExecutionPolicy Bypass -File register-protocol.ps1
#   反注册: powershell -ExecutionPolicy Bypass -File register-protocol.ps1 -Unregister
#
# 说明: 新版打包安装后由 tauri-plugin-deep-link 的 register_all 自动注册，
#       本脚本仅用于旧版本 / 未安装场景的应急注册。

param(
  [string]$ExePath = "e:\code\XDownload\src-tauri\target\release\xdownload.exe",
  [switch]$Unregister
)

$proto = "HKCU:\Software\Classes\xdownload"

if ($Unregister) {
  if (Test-Path $proto) { Remove-Item -Path $proto -Recurse -Force }
  Write-Host "已反注册 xdownload:// 协议"
  exit 0
}

if (-not (Test-Path $ExePath)) {
  Write-Host "错误: 找不到 exe: $ExePath"
  exit 1
}

New-Item -Path "$proto\shell\open\command" -Force | Out-Null
Set-ItemProperty -Path $proto -Name "(default)" -Value "URL:xdownload protocol"
Set-ItemProperty -Path $proto -Name "URL Protocol" -Value ""
New-Item -Path "$proto\DefaultIcon" -Force | Out-Null
Set-ItemProperty -Path "$proto\DefaultIcon" -Name "(default)" -Value "$ExePath,0"
Set-ItemProperty -Path "$proto\shell\open\command" -Name "(default)" -Value "`"$ExePath`" `"%1`""

Write-Host "已注册 xdownload:// 协议 -> $ExePath"
