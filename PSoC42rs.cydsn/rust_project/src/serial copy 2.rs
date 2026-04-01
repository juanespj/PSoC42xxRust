use crate::*;
use core::ffi::c_char;
use core::fmt::{self, Write};
use core::str;
use ffi::*;
// use fixed::types::I32F32; //FixedI32, consts,types::I32F32
use rust_core::encoder_core::SCALE;
use rust_core::serial_core::UartHardware;

pub struct EmbeddedUartHw;

impl UartHardware for EmbeddedUartHw {
    fn put_str(&self, s: &str) {
        uart_put_str(s);
    }

    fn init(&self) {
        unsafe {
            UART_Start();
            UART_SetCustomInterruptHandler(Some(RSUARTRX));
        }
    }

    fn clear_rx_buffer(&self) {
        unsafe {
            UART_SpiUartClearRxBuffer();
        }
    }

    fn put_tx(&self, d: u32) {
        unsafe { UART_SpiUartWriteTxData(d) }
    }

    fn put_crlf(&self, d: u32) {
        unsafe { UART_UartPutCRLF(d) }
    }

    fn put_bytes(&self, bytes: &[u8]) {
        let mut buf = [0u8; 64];
        let len = bytes.len().min(buf.len() - 1);
        buf[..len].copy_from_slice(&bytes[..len]);
        buf[len] = 0;
        unsafe { UART_UartPutString(buf.as_ptr() as *const c_char) }
    }
}

#[unsafe(no_mangle)]
extern "C" fn RSUARTRX() {
    // RX_WAKER.signal(());
    unsafe {
        let ch: u8 = UART_UartGetChar() as u8;
        let cmd = UART.get_mut().parse_byte(ch);
        ClearInterrutpt_RX();
    }
}

// #[macro_export]
// macro_rules! uart_println {
//     ($($arg:tt)*) => {
//         $crate::uart_printf(format_args!($($arg)*));
//     };
// }
#[inline(always)]
pub fn uart_put_tx(d: u32) {
    unsafe { UART.get_mut().put_tx(d) }
}

#[inline(always)]
pub fn uart_put_tx_crlf(d: u32) {
    unsafe { UART.get_mut().put_tx_crlf(d) }
}

#[inline(always)]
pub fn uart_put_str(s: &str) {
    unsafe { UART.get_mut().put_str(s) }
}
#[inline(always)]
pub fn uart_put_bytes(bytes: &[u8]) {
    unsafe { UART.get_mut().put_bytes(bytes) }
}

#[inline(always)]
pub fn uart_send_i32_dec(value: i32) {
    unsafe { UART.get_mut().send_i32_dec(value) }
}
#[inline(always)]
pub fn uart_send_i64_dec(value: i64) {
    unsafe { UART.get_mut().send_i64_dec(value) }
}
// ... and so on for all the other functions

#[inline(always)]
pub fn uart_printf(args: core::fmt::Arguments<'_>) {
    use core::fmt::Write; // <-- Add this import here
    unsafe {
        UART.get_mut().printf(args);
    }
}
