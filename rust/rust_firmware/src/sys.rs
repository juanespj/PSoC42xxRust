use crate::Xaxis;
use crate::ffi::*;
use crate::motor::with_xaxis_mut;
use crate::serial::uart_put_str;
use rs_core::motor::MotorState;
use rs_core::sys::SysHal;

pub use rs_core::sys::{System_State, System_T};

/// Board bindings for the system state machine: console output, the
/// motor-enable pin, and the globally-owned X axis.
pub struct XSysHal;
impl SysHal for XSysHal {
    fn print(&mut self, s: &str) {
        uart_put_str(s);
    }
    fn set_enable(&mut self, on: bool) {
        // EN pin is active-low: 0 enables the driver, 1 disables it.
        unsafe { EN_Write(if on { 0 } else { 1 }) };
    }
    fn motor_state(&self) -> MotorState {
        Xaxis.get().state.clone()
    }
    fn motor_start_motion(&mut self) {
        with_xaxis_mut(|axis| axis.start_motion());
    }
    fn motor_control_stop(&mut self) {
        with_xaxis_mut(|axis| axis.control_stop());
    }
    fn motor_stop(&mut self) {
        with_xaxis_mut(|axis| axis.stop());
    }
}
