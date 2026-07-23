@echo off
setlocal enabledelayedexpansion

echo ============================================
echo   XDownload v1.0.5 Portable Build Script
echo ============================================
echo.

set "PROJECT=%~dp0"
cd /d "%PROJECT%"

:: ===== Step 1: Compile =====
echo [1/5] Compiling...
if exist out\classes rmdir /s /q out\classes
mkdir out\classes
javac -encoding UTF-8 -cp lib\flatlaf-3.5.jar -d out\classes -sourcepath src src/Main.java
if %errorlevel% neq 0 (
    echo [FAIL] Compile failed! Check JDK 17+
    goto :error
)
echo   [OK] Compile done

:: ===== Step 2: Package JAR =====
echo [2/5] Packaging JAR...
if exist out\XDownload.jar del out\XDownload.jar
jar cfe out\XDownload.jar Main -C out\classes .
if %errorlevel% neq 0 (
    echo [FAIL] JAR packaging failed!
    goto :error
)
echo   [OK] JAR: out\XDownload.jar

:: ===== Step 3: Link JRE =====
echo [3/5] Linking custom JRE...
if exist out\runtime rmdir /s /q out\runtime
jlink --no-header-files --no-man-pages --compress=2 ^
    --add-modules java.base,java.logging,jdk.crypto.ec,java.desktop ^
    --output out\runtime
if %errorlevel% neq 0 (
    echo [FAIL] jlink failed! Check JDK 17+
    goto :error
)
echo   [OK] JRE: out\runtime

:: ===== Step 4: jpackage =====
echo [4/5] Creating app-image...
if exist build rmdir /s /q build
mkdir build\jar
copy out\XDownload.jar build\jar\ >nul
if exist lib\flatlaf-3.5.jar copy lib\flatlaf-3.5.jar build\jar\ >nul

jpackage ^
    --type app-image ^
    --name XDownload ^
    --main-class Main ^
    --main-jar XDownload.jar ^
    --input build\jar ^
    --runtime-image out\runtime ^
    --dest build ^
    --vendor "XDownload" ^
    --app-version "1.0.5" ^
    --description "XDownload"
if %errorlevel% neq 0 (
    echo [FAIL] jpackage failed!
    goto :error
)
echo   [OK] app-image: build\XDownload

:: ===== Step 5: Copy binaries =====
echo [5/5] Copying binaries...
set "APP=build\XDownload"
mkdir "%APP%\bin" 2>nul

if exist bin\yt-dlp.exe (
    copy /y bin\yt-dlp.exe "%APP%\bin\" >nul
    echo   [OK] yt-dlp.exe
) else (
    echo   [WARN] bin\yt-dlp.exe not found
)
if exist bin\ffmpeg.exe (
    copy /y bin\ffmpeg.exe "%APP%\bin\" >nul
    echo   [OK] ffmpeg.exe
) else (
    echo   [WARN] bin\ffmpeg.exe not found
)
if exist bin\ffplay.exe copy /y bin\ffplay.exe "%APP%\bin\" >nul
if exist bin\ffprobe.exe copy /y bin\ffprobe.exe "%APP%\bin\" >nul

mkdir "%APP%\config" 2>nul
mkdir "%APP%\downloads" 2>nul

:: ===== Done =====
echo.
echo ============================================
echo   [OK] Build complete!
echo ============================================
echo.
echo   Output: build\XDownload\
echo   Launch: build\XDownload\XDownload.exe
echo.
echo   Copy build\XDownload\ to any location
echo   and run XDownload.exe directly.
echo.
echo   Structure:
echo     XDownload\
echo     +-- XDownload.exe
echo     +-- app\          (main program)
echo     +-- runtime\      (bundled JRE)
echo     +-- bin\          (yt-dlp / ffmpeg)
echo     +-- config\       (settings)
echo     +-- downloads\    (output)

endlocal
pause
exit /b 0

:error
echo.
echo ============================================
echo   [FAIL] Build failed. See errors above.
echo ============================================
endlocal
pause
exit /b 1
