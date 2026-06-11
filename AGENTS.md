# PSoC42xxRust — Agent Guide

## Project structure

Hybrid C/Rust firmware for Cypress PSoC4 (ARM Cortex-M0). Rust is the app entrypoint; C provides the HAL via PSoC Creator-generated code. Rust compiles to a `staticlib` linked into the final ELF by CMake.

**Clone `rs-embedded` as a sibling** (`../../rs-embedded`) — required for `rs-core` and host tests.

```
../../rs-embedded/          # Shared Rust workspace (sibling repo)
├── rs-core/                # motor, sys, ui, encoder_core, serial_core, adrc
└── rs-testiing/            # Host-side testing + egui serial plotter

hal/                        # bindgen_wrappers.h/.c — C↔Rust FFI surface
rust/
├── Cargo.toml              # workspace: rust_firmware + test_ui
└── rust_firmware/          # Firmware staticlib (no_std, thumbv6m-none-eabi)
    └── src/
        ├── lib.rs          # Entrypoint: main(), panic handler, event loop
        ├── motor.rs        # XMotorIo HAL + critical-section wrapper
        ├── encoder.rs      # XEncoder hardware impl (DecL)
        ├── serial.rs       # UART I/O + XMotionCmdHal board bindings
        ├── ui.rs           # XLed / LED_CTRL board bindings
        ├── sys.rs          # XSysHal board bindings
        ├── ffi.rs          # `include!("bindings.rs")` — generated C bindings
        └── host_stubs.rs   # Mock C functions for host compilation

PSoC42rs.cydsn/             # PSoC Creator project + CMake link step
├── cmakebuild.bat          # cargo + CMake/Ninja → .elf
├── rustbuild.bat           # cargo build only (calls ../rust)
└── toolchain-arm-none-eabi.cmake
```

## Build commands (Windows)

```powershell
# Compile Rust staticlib only
cd rust
cargo build -p rust_firmware --target thumbv6m-none-eabi --release

# Or from PSoC42rs.cydsn:
.\rustbuild.bat

# Full build (Rust → CMake/Ninja → .elf)
cd PSoC42rs.cydsn
.\cmakebuild.bat
```

On macOS/Linux (host check only — no ARM link without Creator toolchain):

```bash
./scripts/build_rust.sh
```

## Testing

- Host tests: `cd ../../rs-embedded && cargo test -p rs-testiing`
- `host_stubs.rs` enables `cargo check` on non-ARM targets

## Key architectural constraints

- **No FPU**: Fixed-point math (`fixed` crate, i64 encoder tracking in `rs_core::encoder_core`)
- **Single-core, no OS**: Global state via `local_static`; critical sections via `CyEnterCriticalSection`
- **ISR timing**: `Pulser_InterruptHandler` at 10 µs; main loop ~200 µs per encoder cycle
- **Serial protocol**: Single-char commands + `><var><value>,` parameter assignment
- **`build-std = ["core"]`** with `panic_immediate_abort` in `rust_firmware/.cargo/config.toml`

## C→Rust FFI

- Add symbols to `hal/bindgen_wrappers.h`; `rust_firmware/build.rs` runs bindgen
- Link `hal/bindgen_wrappers.c` via `PSoC42rs.cydsn/CMakeLists.txt`
- PSoC Creator schematic changes require "Generate Application" before rebuild

## Notable quirks

- **Edition mismatch**: `rust_firmware` uses `edition = "2021"`; `rs-core` / `rs-testiing` use `2024`
- **Build environment**: Windows `.bat` scripts; ARM GCC 5.4.1 from PSoC Creator
- **Linker**: `linker_script.ld` / `memory.x` in `rust_firmware/` (when present)
