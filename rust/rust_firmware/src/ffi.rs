#[cfg(target_arch = "arm")]
include!(concat!(env!("OUT_DIR"), "/bindings.rs"));

#[cfg(not(target_arch = "arm"))]
pub use crate::host_stubs::*;
