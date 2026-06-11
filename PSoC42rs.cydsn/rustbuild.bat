@echo off
setlocal enabledelayedexpansion

@REM Run from PSoC42rs.cydsn — builds ../rust/rust_firmware staticlib for CMake link.
set "CYDSN_DIR=%~dp0"
set "REPO_ROOT=%CYDSN_DIR%.."
set "RUST_WS=%REPO_ROOT%\rust"
set "RS_CORE=%REPO_ROOT%\..\rs-embedded\rs-core"
set "RUST_LIB=%RUST_WS%\rust_firmware\build\thumbv6m-none-eabi\release\librust_firmware.a"

@REM Toolchain paths (override via environment if needed)
if not defined PSOC_GNU_PATH (
    set "PSOC_GNU_PATH=C:\Program Files (x86)\Cypress\PSoC Creator\4.4\PSoC Creator\import\gnu\arm\5.4.1"
)
if not defined LIBCLANG_PATH (
    set "LIBCLANG_PATH=C:\Program Files\LLVM\bin"
)

where cargo >nul 2>&1
if errorlevel 1 (
    echo Error: cargo not found on PATH.
    exit /b 1
)

if not exist "%RUST_WS%\Cargo.toml" (
    echo Error: Rust workspace not found at "%RUST_WS%"
    exit /b 1
)

if not exist "%RS_CORE%\Cargo.toml" (
    echo Error: rs-core not found at "%RS_CORE%"
    echo Clone rs-embedded as a sibling of PSoC42xxRust.
    exit /b 1
)

if not exist "%CYDSN_DIR%Generated_Source\PSoC4\project.h" (
    echo Warning: Generated_Source not found — run Generate Application in PSoC Creator first.
    echo Bindgen expects: "%CYDSN_DIR%Generated_Source\PSoC4"
)

echo ============================================
echo Building rust_firmware (thumbv6m-none-eabi)
echo Workspace: %RUST_WS%
echo PSOC_GNU_PATH: %PSOC_GNU_PATH%
echo LIBCLANG_PATH: %LIBCLANG_PATH%
echo ============================================

pushd "%RUST_WS%"
cargo build -p rust_firmware --target thumbv6m-none-eabi --release --verbose
set BUILD_ERR=!errorlevel!
popd

if not !BUILD_ERR! equ 0 (
    echo.
    echo ============================================
    echo Error: cargo build failed with errorlevel: !BUILD_ERR!
    echo ============================================
    exit /b !BUILD_ERR!
)

if not exist "%RUST_LIB%" (
    echo.
    echo Error: expected staticlib missing:
    echo   %RUST_LIB%
    exit /b 1
)

echo.
echo ============================================
echo Build completed successfully!
echo ============================================
echo Output: %RUST_LIB%
echo.

endlocal
exit /b 0
