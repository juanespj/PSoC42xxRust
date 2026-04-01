# Gemini Context: PSoC42xxRust

This project is a hybrid C/Rust firmware implementation for the Cypress PSoC4 (ARM Cortex-M0) platform, specifically the CY8C4245AXI-485. It integrates a Rust-based application core into a legacy PSoC Creator environment, using Rust for high-level logic and C for the hardware abstraction layer (HAL).

## Project Overview

- **Architecture:** The firmware is structured as a Rust `staticlib` that is compiled and then linked into a C project using CMake. The Rust code serves as the primary entry point (`main`), bypassing the traditional C `main` function via linker flags.
- **Technologies:**
    - **PSoC Creator:** Used for schematic design (`TopDesign.cysch`) and generation of the C HAL (`Generated_Source/`).
    - **Rust (`no_std`):** Used for application logic, motor control, and serial protocols.
    - **Bindgen:** Automatically generates Rust FFI bindings from PSoC C headers (via `bindgen_wrappers.h`).
    - **CMake:** Orchestrates the final compilation of C/ASM source and the linking of the Rust static library.

## Building and Running

### Prerequisites
- **ARM GCC Toolchain:** (e.g., v5.4.1 included with PSoC Creator). Path should be set in `PSOC_GNU_PATH`.
- **LLVM/Clang:** Required for `bindgen`. Path should be set in `LIBCLANG_PATH`.
- **Rust Toolchain:** `thumbv6m-none-eabi` target.

### Build Commands
1. **Compile Rust Library:**
   ```powershell
   cd PSoC42rs.cydsn
   .\rustbuild.bat
   ```
   *(This runs `cargo build -p rust_project --release --target thumbv6m-none-eabi`)*

2. **Compile C and Link ELF:**
   ```powershell
   cd PSoC42rs.cydsn
   .\cmakebuild.bat
   ```
   *(This uses CMake and Ninja to produce the final `.elf` and `.hex` files)*

3. **Hardware Changes:**
   If the schematic in PSoC Creator is modified, you must "Generate Application" in the IDE to update the `Generated_Source` before rebuilding.

## Development Conventions

- **Rust Environment:** Strictly `no_std`. Core-only logic with no allocator.
- **Concurrency:** The system is single-core. Global state is managed using the `LocalStatic` pattern (from the `local_static` crate) or `critical-section` to ensure safety without the overhead of a full mutex.
- **Mathematics:** Optimization for the Cortex-M0 is critical. Prefer **fixed-point math** (using the `fixed` crate) over floating-point to avoid heavy software emulation.
- **FFI Boundary:**
    - New C functions or hardware components must be added to `bindgen_wrappers.h` to be visible to Rust.
    - Rust functions exposed to C must use `#[unsafe(no_mangle)] pub extern "C"`.
- **Panic Handling:** A custom panic handler in `src/lib.rs` logs location and message to UART before halting with a breakpoint.

## Key Files
- `PSoC42rs.cydsn/rust_project/src/lib.rs`: Main application logic and hardware task loop.
- `PSoC42rs.cydsn/rust_project/build.rs`: Bindgen configuration and linker script selection.
- `PSoC42rs.cydsn/CMakeLists.txt`: Final link orchestration.
- `PSoC42rs.cydsn/bindgen_wrappers.h`: The bridge header defining the C/Rust FFI boundary.
