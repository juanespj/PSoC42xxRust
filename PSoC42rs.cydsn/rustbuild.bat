
@set GNUPATH=C:\Program Files (x86)\Cypress\PSoC Creator\4.4\PSoC Creator\import\gnu\arm\5.4.1

@REM *** MODIFIED: Cleaned up Clang flags to only include the Clang target triple and includes ***
@bindgen bindgen_wrappers.h --ctypes-prefix "cty" --use-core -o rust_project/src/bindings.rs -- --target=arm -mfloat-abi=soft -mcpu=cortex-m0 -mthumb -Icodegentemp -IGenerated_Source/PSoC4 -I"%GNUPATH%\arm-none-eabi\include" 

@if %errorlevel% neq 0 (
    @echo Error: bindgen failed with errorlevel: %errorlevel%
    @exit /b %errorlevel%
)
@pushd rust_project
@REM This line is essential to compile the resulting bindings.rs file correctly.
@cargo build --target thumbv6m-none-eabi --release --verbose
@if %errorlevel% neq 0 (
    @echo Error: bindgen failed with errorlevel: %errorlevel%
    @exit /b %errorlevel%
)
cd /d "%~dp0"

@REM to create rust api for C
@REM cbindgen --crate rust_project --output rust_api.h
@popd