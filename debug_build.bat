@echo off
chcp 65001 >nul
set ROOT=%~dp0

echo ========================================
echo   XDownload - Debug Build (fast)
echo ========================================
echo   mode: debug, no bundle (no installer)
echo.

cd /d "%ROOT%"
node ui/node_modules/@tauri-apps/cli/tauri.js build --debug --no-bundle
if %errorlevel% neq 0 (
    echo.
    echo ERROR: Debug build failed
    pause
    exit /b 1
)

echo.
echo Done! Output: %ROOT%target\debug\xdownload.exe
pause
