#[cfg(target_arch = "arm")]
// On the ARM target, the 'bindings' module is defined by the generated C code.
// We use `mod bindings;` if bindings.rs is auto-generated and placed in src/.
// If you use `include!`, you would use it *inside* the module block below,
// but often `mod` is cleaner if `bindings.rs` can be a standalone file.
include!("bindings.rs");

#[cfg(not(target_arch = "arm"))]
// On the host, the 'bindings' module is defined by the stub file.
// We define the module using the contents of the stubs file.
// Ensure the stubs are defined in src/bindings.rs and the stubs file
// itself doesn't have its own `#![cfg]`
include!("host_stubs.rs");
