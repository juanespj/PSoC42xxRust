// use crate::RX_WAKER;
use crate::*;
use core::ffi::c_char;
use core::fmt::{self, Write};
use core::str;
use ffi::*;

// rust_project/src/uart.rs
use crate::ffi::*;
use crate::{MotorDirection, SYS, Xaxis};
use core::ffi::c_char;
use rust_core::commands::{Command, parse_byte};
use rust_core::serial::FmtWriter;
use rust_core::serial::SerialWrite;

pub struct Uart;

impl SerialWrite for Uart {
    fn write_bytes(&self, bytes: &[u8]) {
        let mut buf = [0u8; 64];
        let len = bytes.len().min(buf.len() - 1);
        buf[..len].copy_from_slice(&bytes[..len]);
        buf[len] = 0;
        unsafe {
            UART_UartPutString(buf.as_ptr() as *const c_char);
        }
    }
}

#[unsafe(no_mangle)]
extern "C" fn RSUARTRX() {
    unsafe {
        let ch = UART_UartGetChar() as u8;
        on_rx_byte(ch);
        ClearInterrutpt_RX();
    }
}

pub fn on_rx_byte(ch: u8) {
    let uart = Uart;

    match parse_byte(ch) {
        Some(Command::ToggleDir) => {
            let dir = if Xaxis.get_mut().dir == MotorDirection::BWD {
                MotorDirection::FWD
            } else {
                MotorDirection::BWD
            };
            Xaxis.get_mut().set_direction(dir);
        }

        Some(Command::Kill) => SYS.get_mut().next_state = System_State::KILL,
        Some(Command::ToggleDebug) => SYS.get_mut().print_dbg ^= true,
        Some(Command::StartMove) => SYS.get_mut().next_state = System_State::START_MOVE,
        Some(Command::StartSpeed) => SYS.get_mut().next_state = System_State::START_SPD,
        Some(Command::Stop) => SYS.get_mut().next_state = System_State::STOPPING,

        Some(Command::Unknown(b)) => {
            let mut w = FmtWriter(&uart);
            let _ = write!(w, "\n\rDBG: {} [{}]", b, b.escape_ascii());
        }

        None => {}
    }
}

pub fn UI_init() {
    unsafe {
        UART_Start();
        UART_SetCustomInterruptHandler(Some(RSUARTRX));
        UART_SpiUartClearRxBuffer();
    }
    term(TermCmd::ClearScreen);
    term(TermCmd::Home);
    uart_put_str("\n\r-PSOC RS\r\n");
}
#[unsafe(no_mangle)]
extern "C" fn RSUARTRX() {
    // RX_WAKER.signal(());
    unsafe {
        let ch: u32 = UART_UartGetChar();
        serial_event(ch);
        ClearInterrutpt_RX();
    }
}
