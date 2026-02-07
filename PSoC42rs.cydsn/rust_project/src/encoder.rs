pub struct XEncoder;
use crate::ffi::*;
use crate::uart_println;
use crate::utils::{IirFilter, RingBuf};
use crate::Config::*;
use crate::Xaxis;
use crate::SYS;
use bitfield_struct::bitfield;
use fixed::{consts, types::I16F16, types::I32F32, FixedI32};
use rust_core::encoder_core::{Encoder, EncoderOps};

impl EncoderOps for XEncoder {
    fn init_hardware(&self) {
        unsafe {
            DecL_Init();
            DecL_Start();
            #[cfg(target_arch = "arm")]
            ISR_DecL_StartEx(Some(XaxisEncoder_InterruptHandler));
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

#[unsafe(no_mangle)]
extern "C" fn XaxisEncoder_InterruptHandler() {
    unsafe {
        // let count = Xaxis.get_mut().encoder.read_counter();
        // Xaxis.get_mut().encoder.counts.push(count);
        // Xaxis.get_mut().encoder.update();
        #[cfg(target_arch = "arm")]
        ISR_DecL_ClearPending();
    }
}

fn counts_to_theta(counts: i32) -> I16F16 {
    I16F16::from_num(counts) * RAD_TO_COUNTS
}
