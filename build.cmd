@echo off
setlocal
:: Check for Go
where go >nul 2>nul
if errorlevel 1 (
    echo [ERROR] Go is not installed or not in PATH.
    echo Please install Go from https://golang.org/dl/
    pause
    exit /b 1
)
:: Set paths
set "SRCDIR=%~dp0%src"
set "OUTFILE=TurboRipent"
set "HASHFILE=%OUTFILE%.sha256.txt"
:: Build
go build -o "%OUTFILE%.exe" "%SRCDIR%"
if errorlevel 1 (
    echo [ERROR] Build failed.
    pause
    exit /b %ERRORLEVEL%
)

echo [OK] Build succeeded: %OUTFILE%
:: Generate SHA256 checksum and save to file
CertUtil -hashfile "%OUTFILE%.exe" SHA256 > "%HASHFILE%"
echo [INFO] SHA256 checksum saved to: %HASHFILE%
type %HASHFILE%
:: --- Generate the minimised launcher CMD ---
(
    echo @echo off
    echo "%%~dp0%OUTFILE%.exe" -edit "%%~1"
) > "%OUTFILE%-Editor.cmd"
echo [INFO] Created launcher: %OUTFILE%-Editor.cmd

endlocal
pause
