pub struct XEncoder;
use crate::utils::{IirFilter, RingBuf};
use crate::Config::*;
use crate::{ffi::*, Xaxis};
use rust_core::encoder_core::{Encoder, EncoderOps};

impl EncoderOps for XEncoder {
    fn init_hardware(&self) {
        unsafe {
            DecL_Init();
            DecL_Start();
            // #[cfg(target_arch = "arm")]
            // ISR_DecL_StartEx(Some(XaxisEncoder_InterruptHandler));
            Pulser_tmr_Start();
        }
    }
    fn start_hardware(&self) {
        unsafe { DecL_Start() }
    }
    fn write_counter(&self, value: u32) {
        unsafe { DecL_WriteCounter(value) }
    }
    #[inline(always)]
    fn get_counter(&self) -> u32 {
        unsafe { DecL_ReadCounter() } // call your C binding
    }
}

// #[unsafe(no_mangle)]
// extern "C" fn XaxisEncoder_InterruptHandler() {
//     unsafe {

//         #[cfg(target_arch = "arm")]
//         ISR_DecL_ClearPending();
//     }
// }
