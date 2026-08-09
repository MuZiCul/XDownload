@echo off
chcp 65001 >nul
set ROOT=%~dp0

echo ========================================
echo   XDownload v2.7.1 - Build ^& Package
echo ========================================
echo.

echo Building (frontend + Rust + installer)...
echo tauri.conf.json beforeBuildCommand will auto-build the frontend first.
echo.

cd /d "%ROOT%src-tauri"
cargo tauri build
if %errorlevel% neq 0 (
    echo.
    echo ERROR: Build failed
    pause
    exit /b 1
)

echo.
echo ========================================
echo Done! Output:
echo ========================================
dir /b /s "%ROOT%src-tauri\target\release\bundle\nsis\*.exe" 2>nul
dir /b /s "%ROOT%src-tauri\target\release\bundle\msi\*.msi" 2>nul
echo.
pause
