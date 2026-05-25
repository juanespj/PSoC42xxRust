# PSoC42xxRust — Agent Guide

## Project structure

Hybrid C/Rust firmware for Cypress PSoC4 (ARM Cortex-M0). Rust is the app entrypoint; C provides the HAL via PSoC Creator-generated code. Rust compiles to a `staticlib` linked into the final ELF by CMake.

```
PSoC42rs.cydsn/
├── rust_project/       # Firmware staticlib (no_std, thumbv6m-none-eabi)
│   └── src/
│       ├── lib.rs      # Entrypoint: main(), panic handler, event loop
│       ├── motor.rs    # Stepper motor state machine, Pulser ISR
│       ├── encoder.rs  # XEncoder hardware impl (DecL)
│       ├── serial.rs   # UART I/O, terminal commands
│       ├── ui.rs       # LED, debounced button
│       ├── sys.rs      # System state machine (IDLE/MOVING/SPEED/KILL…)
│       ├── utils.rs    # IIR filter, RingBuf
│       ├── Config.rs   # Fixed-point constants for encoder filters
│       ├── ffi.rs      # `include!("bindings.rs")` — generated C bindings
│       └── host_stubs.rs # Mock C functions for host compilation
├── rust_core/          # Shared library (no_std, used by firmware + testing)
│   └── src/
│       ├── adrc.rs          # 1st/2nd-order LADRC controller (i64 fixed-point)
│       ├── encoder_core.rs  # Encoder struct + tracking (i64-based, NOT I32F32)
│       ├── serial_core.rs   # SerialParser, Command enum
│       └── utils_core.rs    # IIR filter, RingBuf (u64-based)
├── rs_testing/         # Host-side testing + egui serial plotter
├── test_ui/            # Separate host UI with async serial
├── cmakebuild.bat      # Full build: cargo + CMake/Ninja → .elf
├── rustbuild.bat       # Cargo build only (staticlib)
├── build.rs            # Bindgen config
├── bindgen_wrappers.h  # C→Rust FFI bridge header
└── toolchain-arm-none-eabi.cmake
```

## Build commands (Windows)

```powershell
# Build firmware staticlib only
cargo build -p rust_project --target thumbv6m-none-eabi --release

# Or use aliases:
cargo build-firmware        # defined in .cargo/config.toml

# Run host-side tests
cargo test -p rs_testing
cargo test-host             # alias

# Full build (Rust → CMake/Ninja → .elf)
.\cmakebuild.bat            # calls rustbuild.bat internally
```

## Testing

- All tests run on host (`cargo test -p rs_testing`). No on-device tests.
- `rs_testing` uses `gnuplot` for visualization — `cargo test` will pop gnuplot windows.
- `host_stubs.rs` provides mock C functions for host compilation (only when `cfg(not(target_arch = "arm"))`).
- Encoder filter tuning tests live in `rs_testing/src/encoder_tests.rs`.

## Key architectural constraints

- **No FPU**: All math is fixed-point (`fixed` crate: `I16F16`, `I32F32`). The active encoder uses custom `i64`-based tracking (not `I32F32`) in `rust_core::encoder_core`.
- **Single-core, no OS**: Global state via `local_static` crate. Critical sections use `CyEnterCriticalSection`/`CyExitCriticalSection` in ISR-boundary code.
- **ISR timing**: `Pulser_InterruptHandler` fires at 10 µs intervals, runs the stepper `run()` method. The main loop runs at ~200 µs per encoder update cycle.
- **Fixed-point constants** (`Config.rs`, `encoder_core.rs`) are expressed as `from_bits()` values. Tuning filter gains (`g_a`, `g_b`, `g_c`) is done at runtime via serial commands (`>a<value>`, `>b<value>`, `>c<value>`).
- `build-std = ["core"]` with `panic_immediate_abort` enabled in `rust_project/.cargo/config.toml`.

## C→Rust FFI

- New C functions/components must be added to `bindgen_wrappers.h` and regenerated (`cargo build` runs bindgen automatically via `build.rs`).
- Rust functions exposed to C use `#[unsafe(no_mangle)] pub extern "C"`.
- Generated bindings go to `$OUT_DIR/bindings.rs`, pulled in by `ffi.rs` via `include!("bindings.rs")`.

## Notable quirks

- **Edition mismatch**: `rust_project` uses `edition = "2021"`; `rust_core`, `rs_testing`, `test_ui` use `edition = "2024"`.
- **Cargo workspace resolver**: `resolver = "2"` at `PSoC42rs.cydsn/Cargo.toml`.
- **Outdated backup files** exist (`serial copy.rs`, `serial copy 2.rs`, `encoder_core I32F32.rs`, `libembassy.rs`) — not part of the build.
- **Build environment is Windows** (`.bat` scripts). The ARM GCC 5.4.1 from PSoC Creator is expected at `C:\Program Files (x86)\Cypress\PSoC Creator\4.4\...`.
- **Linker**: Custom `linker_script.ld` and `memory.x` in `rust_project/`. The `build.rs` passes `-Tlink.x -nostartfiles -Tlinker_script.ld`.
- **PSoC Creator schematic changes** require "Generate Application" in the IDE before rebuild to update `Generated_Source/`.
- **Serial terminal protocol**: Single-character commands (`r`=start move, `s`=start speed, `k`=kill, `t`=stop, `d`=toggle dir, `z`=reset encoder). Parameter assignment via `><char><value>,` (e.g. `>a123,` sets gain A).
- **Speed reference** comes from ADC (potentiometer) sample read every ~3 main-loop iterations, set via `axis.set_speed(spd_ref)`.
