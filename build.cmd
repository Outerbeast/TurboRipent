@echo off
setlocal
:: Check for Rust installation
where cargo >nul 2>nul
if errorlevel 1 (
    echo Rust is not installed. Please install Rust from https://rustup.rs/
    exit /b 1
)
:: Build the project in release mode
echo Building project...
cargo build --release
:: Check if build succeeded
if errorlevel 1 (
    echo Build failed.
    exit /b 1
)
:: Optional: copy binary to a friendly location
set BIN_NAME=TurboRipent
set DEST=%~dp0

mkdir %DEST% >nul 2>nul
copy target\release\%BIN_NAME%.exe %DEST% >nul
CertUtil -hashfile "%~dp0%BIN_NAME%.exe" SHA256 > "%~dp0%BIN_NAME%.exe.sha256.txt"

echo Build complete. The executable is located at %DEST%\%BIN_NAME%.exe
type "%~dp0%BIN_NAME%.exe.sha256.txt"
endlocal
