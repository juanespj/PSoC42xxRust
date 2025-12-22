@REM Update this folder to match your install path
@set TOOLCHAIN_PREFIX=C:/Program Files (x86)/Cypress/PSoC Creator/4.4/PSoC Creator/import/gnu/arm/5.4.1
@REM ----------------------------------------------------------------------------------
@REM STEP 1: Run bindgen and cargo build. If this fails, the script will likely stop.
@REM Ensure the rustbuild.bat is fixed with the 64-bit LIBCLANG_PATH.
@echo Running rustbuild.bat...

@CALL rustbuild.bat
@REM Check the exit code of the last command (rustbuild.bat)
@echo rustbuild.bat exited with errorlevel: %errorlevel%
@REM Check the exit code of the last command (rustbuild.bat)
@if %errorlevel% neq 0 (
    @echo Error: rustbuild.bat failed with errorlevel: %errorlevel%
    @exit /b %errorlevel%
)

@REM Clear out our build folder, if it exists
@echo Cleaning build directory...
@if exist build rmdir build /Q /S
@echo Creating build directory...
@mkdir build

@REM Do the build!
@pushd build
@echo Running CMake configuration...
@cmake .. -GNinja -DCMAKE_BUILD_TYPE=Release -DCMAKE_TOOLCHAIN_FILE=../toolchain-arm-none-eabi.cmake -DTOOLCHAIN_PREFIX="%TOOLCHAIN_PREFIX%"

@echo Running Ninja build...
@ninja


