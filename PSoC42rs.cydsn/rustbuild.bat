@echo off
setlocal enabledelayedexpansion

@REM Set paths (optional - can also set as system environment variables)
set "PSOC_GNU_PATH=C:\Program Files (x86)\Cypress\PSoC Creator\4.4\PSoC Creator\import\gnu\arm\5.4.1"
set "LIBCLANG_PATH=C:\Program Files\LLVM\bin"

@REM Build the firmware project from workspace root
echo Building rust_project from workspace...
cargo build -p rust_project --target thumbv6m-none-eabi --release --verbose

if %errorlevel% neq 0 (
    echo.
    echo ============================================
    echo Error: Build failed with errorlevel: %errorlevel%
    echo ============================================
    exit /b %errorlevel%
)

echo.
echo ============================================
echo Build completed successfully!
echo ============================================
echo Output: target\thumbv6m-none-eabi\release\rust_project.elf
echo.

@REM Optional: Generate C API header
@REM echo Generating C API header...
@REM cbindgen --crate rust_project --output rust_api.h

endlocal