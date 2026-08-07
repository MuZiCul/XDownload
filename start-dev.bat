@echo off
rem ============================================
rem  XDownload one-click dev launcher (start-dev.bat)
rem  Steps: cleanup leftovers -> compile check -> start dev
rem  NOTE: keep this file pure ASCII (no Chinese, no chcp 65001)
rem        to avoid cmd codepage issues on any Windows locale.
rem ============================================
set ROOT=%~dp0
cd /d "%ROOT%"

echo ========================================
echo   XDownload - one-click dev launcher
echo ========================================
echo.

rem ---------- 1. Cleanup ----------
echo [1/3] Cleaning up leftover processes...
echo   Will terminate:
echo     - xdownload.exe (leftover app process)
echo     - process occupying port 1420 (Vite dev server)
echo   Press Ctrl+C to abort if you don't want this.
echo.
pause

rem Kill XDownload app process; ignore "not found" errors
taskkill /F /IM xdownload.exe >nul 2>&1

rem Kill the process occupying port 1420 via PowerShell
powershell -NoProfile -Command "Get-NetTCPConnection -LocalPort 1420 -ErrorAction SilentlyContinue | ForEach-Object { Stop-Process -Id $_.OwningProcess -Force -ErrorAction SilentlyContinue }"
echo   Cleanup done.
echo.

rem ---------- 2. Compile check ----------
echo [2/3] Compile check...

rem Frontend type check
echo.
echo   -- Frontend type check (npx tsc --noEmit) --
cd /d "%ROOT%ui"
call npx tsc --noEmit
if errorlevel 1 (
    echo.
    echo   [ERROR] Frontend TypeScript check failed. See output above.
    pause
    cd /d "%ROOT%"
    exit /b 1
)
echo   Frontend check passed.

rem Backend check
echo.
echo   -- Backend check (cargo check) --
cd /d "%ROOT%src-tauri"
call cargo check
if errorlevel 1 (
    echo.
    echo   [ERROR] Backend Rust check failed. See output above.
    pause
    cd /d "%ROOT%"
    exit /b 1
)
echo   Backend check passed.
echo.

rem Back to project root
cd /d "%ROOT%"

rem ---------- 3. Start dev ----------
echo [3/3] Starting dev...
echo   Running: pnpm dev
echo   This starts the Vite frontend + Tauri backend and runs long-term.
echo   Keep this window open; press Ctrl+C to stop.
echo.
pause

pnpm dev

echo.
echo   App exited. Press any key to close.
echo.
pause
