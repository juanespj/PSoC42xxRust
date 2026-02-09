// use crate::RX_WAKER;
use crate::*;
use crate::{SYS, UART};
use core::ffi::c_char;
use core::fmt::{self, Write};

use core::str;
use ffi::*;

use rust_core::serial_core::{parse_byte, term, Command, FmtWriter, SerialWrite, TermCmd};
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

    fn write_str(&self, s: &str) {
        uart_put_str(s);
    }
    fn write_fmt(&self, args: fmt::Arguments<'_>) {
        let mut writer = FmtWriter(self);
        let _ = fmt::write(&mut writer, args);
    }
}
impl Uart {
    pub fn new() -> Self {
        Uart {}
    }
    pub fn UI_init(&mut self) {
        unsafe {
            UART_Start();
            UART_SetCustomInterruptHandler(Some(RSUARTRX));
            UART_SpiUartClearRxBuffer();
        }
        term(self, TermCmd::ClearScreen);
        term(self, TermCmd::Home);
        uart_put_str("\n\r-PSOC RS\r\n");
    }
}

#[inline(always)]
pub fn uart_printf(args: fmt::Arguments<'_>) {
    UART.get_mut().write_fmt(args);
}
#[inline(always)]
pub fn uart_put_str(str: &str) {
    UART.get_mut().write_str(str);
}
#[macro_export]
macro_rules! uart_println {
    ($($arg:tt)*) => {
        $uart.getmut::uart_printf(format_args!($($arg)*));
    };
}

#[unsafe(no_mangle)]
extern "C" fn RSUARTRX() {
    // RX_WAKER.signal(());
    unsafe {
        let ch = UART_UartGetChar() as u8;
        on_rx_byte(ch);
        ClearInterrutpt_RX();
    }
}

pub fn on_rx_byte(ch: u8) {
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
        Some(Command::ToggleDebug) => SYS.get_mut().print_dbg ^= 1,
        Some(Command::StartMove) => SYS.get_mut().next_state = System_State::START_MOVE,
        Some(Command::StartSpeed) => SYS.get_mut().next_state = System_State::START_SPD,
        Some(Command::Stop) => SYS.get_mut().next_state = System_State::STOPPING,

        Some(Command::Unknown(b)) => {
            uart_printf(format_args!("\n\rDBG: {} [{}]", b, b.escape_ascii()));
        }

        None => {}
    }
}
