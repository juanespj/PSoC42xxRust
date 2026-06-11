use crate::SYS;
use crate::UART;
use crate::Xaxis;
use crate::motor::{with_xaxis_mut, AdrcMode, MotorDirection};
use crate::sys::System_State;
use crate::*;
use core::ffi::c_char;
use core::fmt::{self, Write};
use crate::ffi::{
    ClearInterrutpt_RX, UART_SetCustomInterruptHandler, UART_SpiUartClearRxBuffer, UART_Start,
    UART_UartGetChar, UART_UartPutString,
};
use fixed::types::I32F32;
use rs_core::serial_core::{
    dispatch_command, send_i32_dec, send_i64_dec, send_u32_dec, term, ByteSink, MotionCmdHal,
    SerialParser, TermCmd, UartCmdOut,
};

/// UART TX byte sink used by the shared `rs_core` number formatters.
struct TxSink;
impl ByteSink for TxSink {
    #[inline(always)]
    fn put_byte(&mut self, byte: u8) {
        uart_put_tx(byte as u32);
    }
}

/// Routes `dispatch_command` acknowledgements through the global UART instance.
struct CmdOut;
impl UartCmdOut for CmdOut {
    fn write_fmt(&mut self, args: fmt::Arguments) {
        uart_printf(args);
    }
}

/// Board bindings for serial command actions (motor, system state, encoder).
pub struct XMotionCmdHal;
impl MotionCmdHal for XMotionCmdHal {
    fn set_encoder_gain_a(&mut self, value: u64) {
        with_xaxis_mut(|axis| axis.encoder.g_a = value);
    }
    fn set_encoder_gain_b(&mut self, value: u64) {
        with_xaxis_mut(|axis| axis.encoder.g_b = value);
    }
    fn set_encoder_gain_c(&mut self, value: u64) {
        with_xaxis_mut(|axis| axis.encoder.g_c = value);
    }
    fn adrc_update_w0(&mut self, value: u64) {
        with_xaxis_mut(|axis| {
            axis.adrc_update_w0(value);
            if axis.adrc_mode != AdrcMode::Off {
                let mode = axis.adrc_mode;
                axis.adrc_set_mode(mode);
            }
        });
    }
    fn adrc_update_wc(&mut self, value: u64) {
        with_xaxis_mut(|axis| {
            axis.adrc_update_wc(value);
            if axis.adrc_mode != AdrcMode::Off {
                let mode = axis.adrc_mode;
                axis.adrc_set_mode(mode);
            }
        });
    }
    fn adrc_update_b0(&mut self, value: u64) {
        with_xaxis_mut(|axis| {
            axis.adrc_update_b0(value);
            if axis.adrc_mode != AdrcMode::Off {
                let mode = axis.adrc_mode;
                axis.adrc_set_mode(mode);
            }
        });
    }
    fn adrc_set_mode(&mut self, value: u64) {
        let mode = match value {
            1 => AdrcMode::Speed,
            2 => AdrcMode::Position,
            _ => AdrcMode::Off,
        };
        with_xaxis_mut(|axis| axis.adrc_set_mode(mode));
    }
    fn set_target_pos_steps(&mut self, value: u64) {
        with_xaxis_mut(|axis| axis.target_pos_steps = value as i32);
    }
    fn set_target_speed_hz(&mut self, value: u64) {
        with_xaxis_mut(|axis| {
            axis.target_speed_hz = value as i64;
            axis.curr_target_speed_hz = axis.dir_sign() * value as i64;
        });
    }
    fn current_motor_dir_is_bwd(&self) -> bool {
        Xaxis.get().dir == MotorDirection::BWD
    }
    fn set_motor_dir(&mut self, forward: bool) {
        let dir = if forward {
            MotorDirection::FWD
        } else {
            MotorDirection::BWD
        };
        with_xaxis_mut(|axis| axis.set_direction(dir));
    }
    fn system_kill(&mut self) {
        SYS.get_mut().next_state = System_State::KILL;
    }
    fn system_toggle_debug(&mut self) -> bool {
        let sys = SYS.get_mut();
        sys.print_dbg = if sys.print_dbg != 0 { 0 } else { 1 };
        sys.print_dbg != 0
    }
    fn system_start_move(&mut self) {
        SYS.get_mut().next_state = System_State::START_MOVE;
    }
    fn system_start_speed(&mut self) {
        SYS.get_mut().next_state = System_State::START_SPD;
    }
    fn system_stop(&mut self) {
        SYS.get_mut().next_state = System_State::STOPPING;
    }
    fn encoder_zero(&mut self) {
        with_xaxis_mut(|axis| axis.encoder.zero());
    }
}

pub struct UartIf {
    parser: SerialParser,
}

impl Write for UartIf {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        uart_put_str(s);
        Ok(())
    }
}

impl UartIf {
    pub fn new() -> Self {
        Self {
            parser: SerialParser::new(),
        }
    }
    pub fn UI_init(&mut self) {
        unsafe {
            UART_Start();
            UART_SetCustomInterruptHandler(Some(RSUARTRX));
            UART_SpiUartClearRxBuffer();
        }
        term(&mut CmdOut, TermCmd::ClearScreen);
        term(&mut CmdOut, TermCmd::Home);
        uart_put_str("\n\r-PSOC RS\r\n");
    }
    fn serial_event(&mut self, in32: u32) {
        let ch: u8 = (in32 & 0xFF) as u8;
        if let Some(cmd) = self.parser.parse_byte(ch) {
            dispatch_command(&mut CmdOut, &mut XMotionCmdHal, cmd);
        }
    }
}

#[inline(always)]
pub fn uart_printf(args: core::fmt::Arguments<'_>) {
    UART.get_mut().write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! uart_println {
    ($($arg:tt)*) => {
        $crate::uart_printf(format_args!($($arg)*));
    };
}

#[unsafe(no_mangle)]
extern "C" fn RSUARTRX() {
    unsafe {
        let ch: u32 = UART_UartGetChar();
        UART.get_mut().serial_event(ch);
        ClearInterrutpt_RX();
    }
}

pub fn uart_put_bytes(bytes: &[u8]) {
    let mut buf = [0u8; 64];
    let len = bytes.len().min(buf.len() - 1);
    buf[..len].copy_from_slice(&bytes[..len]);
    buf[len] = 0;
    unsafe { UART_UartPutString(buf.as_ptr() as *const c_char) }
}

#[inline(always)]
pub fn uart_put_tx(d: u32) {
    unsafe { UART_SpiUartWriteTxData(d) }
}

#[inline(always)]
pub fn uart_put_tx_crlf(d: u32) {
    unsafe { UART_UartPutCRLF(d) }
}

#[inline(always)]
pub fn uart_put_str(s: &str) {
    uart_put_bytes(s.as_bytes())
}

pub fn uart_send_i32f32_binary(value: I32F32) {
    let bits = value.to_bits();
    uart_put_tx((bits & 0xFF) as u32);
    uart_put_tx(((bits >> 8) & 0xFF) as u32);
    uart_put_tx(((bits >> 16) & 0xFF) as u32);
    uart_put_tx(((bits >> 24) & 0xFF) as u32);
}

pub fn uart_send_u32_dec(value: u32) {
    send_u32_dec(&mut TxSink, value);
}

pub fn uart_send_i32_dec(value: i32) {
    send_i32_dec(&mut TxSink, value);
}

pub fn uart_send_i64_dec(value: i64) {
    send_i64_dec(&mut TxSink, value);
}

pub fn uart_send_i32(value: i32) {
    uart_put_tx_crlf(value as u32);
}

pub fn uart_send_u32_hex(value: u32) {
    use core::fmt::Write;
    write!(UART.get_mut(), "0x{:08X}", value).ok();
}
