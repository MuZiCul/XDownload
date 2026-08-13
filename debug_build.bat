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
echo Build OK: %ROOT%target\debug\xdownload.exe

echo.
echo Stopping running app (if any) ...
taskkill /f /im xdownload.exe >nul 2>&1
timeout /t 1 /nobreak >nul

echo Copying to D:\XDownload ...
set DEST=D:\XDownload
if not exist "%DEST%" (
    echo ERROR: %DEST% does not exist
    pause
    exit /b 1
)
copy /y "%ROOT%target\debug\xdownload.exe" "%DEST%\xdownload.exe" >nul
if errorlevel 1 (
    echo ERROR: failed to copy xdownload.exe
    pause
    exit /b 1
)
copy /y "%ROOT%target\debug\xdownload_lib.dll" "%DEST%\xdownload_lib.dll" >nul
if errorlevel 1 (
    echo ERROR: failed to copy xdownload_lib.dll
    pause
    exit /b 1
)
echo Copied to %DEST%

echo.
echo Starting %DEST%\xdownload.exe ...
start "" "%DEST%\xdownload.exe"
echo Started.
pause
