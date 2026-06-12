@echo off
setlocal enabledelayedexpansion

@REM Full firmware build: Rust staticlib (rustbuild.bat) + CMake/Ninja ELF.
set "CYDSN_DIR=%~dp0"
set "REPO_ROOT=%CYDSN_DIR%.."
set "RUST_LIB=%REPO_ROOT%\rust\target\thumbv6m-none-eabi\release\librust_firmware.a"
set "CMAKE_BUILD_DIR=%CYDSN_DIR%build"
set "ELF_FILE=%CMAKE_BUILD_DIR%\PSoC4rs.elf"

@REM ARM GCC from PSoC Creator (override TOOLCHAIN_PREFIX if installed elsewhere)
if not defined TOOLCHAIN_PREFIX (
    set "TOOLCHAIN_PREFIX=C:/Program Files (x86)/Cypress/PSoC Creator/4.4/PSoC Creator/import/gnu/arm/5.4.1"
)

@REM Keep PSOC_GNU_PATH in sync for rust_firmware bindgen (build.rs)
if not defined PSOC_GNU_PATH (
    set "PSOC_GNU_PATH=%TOOLCHAIN_PREFIX%"
)

echo ============================================
echo Step 1/2: Rust staticlib (rustbuild.bat)
echo ============================================
call "%CYDSN_DIR%rustbuild.bat"
set RUST_ERR=!errorlevel!
if not !RUST_ERR! equ 0 (
    echo Error: rustbuild.bat failed with errorlevel: !RUST_ERR!
    exit /b !RUST_ERR!
)

if not exist "%RUST_LIB%" (
    echo Error: Rust staticlib not found after rustbuild:
    echo   %RUST_LIB%
    exit /b 1
)

if not exist "%CYDSN_DIR%Generated_Source\PSoC4" (
    echo Error: Generated_Source\PSoC4 missing — Generate Application in PSoC Creator first.
    exit /b 1
)

if not exist "%REPO_ROOT%\hal\bindgen_wrappers.c" (
    echo Error: hal\bindgen_wrappers.c not found at repo root.
    exit /b 1
)

where cmake >nul 2>&1
if errorlevel 1 (
    echo Error: cmake not found on PATH.
    exit /b 1
)

where ninja >nul 2>&1
if errorlevel 1 (
    echo Error: ninja not found on PATH.
    exit /b 1
)

echo.
echo ============================================
echo Step 2/2: CMake + Ninja (C HAL + link)
echo ============================================
echo Cleaning %CMAKE_BUILD_DIR% ...
if exist "%CMAKE_BUILD_DIR%" rmdir /s /q "%CMAKE_BUILD_DIR%"
mkdir "%CMAKE_BUILD_DIR%"

pushd "%CMAKE_BUILD_DIR%"
cmake .. -GNinja -DCMAKE_BUILD_TYPE=Release -DCMAKE_TOOLCHAIN_FILE=../toolchain-arm-none-eabi.cmake -DTOOLCHAIN_PREFIX="%TOOLCHAIN_PREFIX%"
set CMAKE_ERR=!errorlevel!
if not !CMAKE_ERR! equ 0 (
    popd
    echo Error: cmake configuration failed with errorlevel: !CMAKE_ERR!
    exit /b !CMAKE_ERR!
)

ninja
set NINJA_ERR=!errorlevel!
popd

if not !NINJA_ERR! equ 0 (
    echo Error: ninja build failed with errorlevel: !NINJA_ERR!
    exit /b !NINJA_ERR!
)

if exist "%ELF_FILE%" (
    echo.
    echo === ELF Analytics ===
    "%TOOLCHAIN_PREFIX%/bin/arm-none-eabi-size.exe" "%ELF_FILE%"
    echo.
    echo Output: %ELF_FILE%
) else (
    echo Warning: expected ELF not found at %ELF_FILE%
)

echo.
echo ============================================
echo cmakebuild completed successfully
echo ============================================

endlocal
exit /b 0
