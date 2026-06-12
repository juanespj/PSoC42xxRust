use crate::encoder::XEncoder;
use crate::ffi::*;
use crate::serial::uart_printf;
use crate::{SYS, Xaxis};

pub use rs_core::motor::{AdrcMode, MotorDirection, MotorIo, MotorState, Stepper};

/// Direction-pin hardware binding for the X axis.
pub struct XMotorIo;
impl MotorIo for XMotorIo {
    fn write_dir(&self, backward: bool) {
        unsafe { DIR_Write(if backward { 1 } else { 0 }) };
    }
}

pub fn with_xaxis_mut<R>(f: impl FnOnce(&mut Stepper<XEncoder, XMotorIo>) -> R) -> R {
    unsafe {
        #[cfg(target_arch = "arm")]
        {
            let saved_intr = CyEnterCriticalSection();
            let result = f(Xaxis.get_mut());
            CyExitCriticalSection(saved_intr);
            result
        }
        #[cfg(not(target_arch = "arm"))]
        {
            f(Xaxis.get_mut())
        }
    }
}

#[unsafe(no_mangle)]
extern "C" fn Pulser_InterruptHandler() {
    //10us
    unsafe {
        Xaxis.get_mut().run(&mut SYS.get_mut().step_out);
        StepReg_Write(SYS.get().step_out as u8);
        Pulser_tmr_ClearInterrupt(Pulser_tmr__INTR_MASK_TC); /* Clears the Timer terminal count interrupt */
    }
}

pub fn pulser_init() {
    unsafe {
        ISR_Pulser_StartEx(Some(Pulser_InterruptHandler));
        Pulser_tmr_Start();
    }
    uart_printf(format_args!("pulser initialized.\n\r"));
}
