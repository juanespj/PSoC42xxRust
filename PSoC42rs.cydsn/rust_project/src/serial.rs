// use crate::RX_WAKER;
use crate::SYS;
use crate::UART;

use crate::*;
use core::ffi::c_char;
use core::fmt::{self, Write};
use core::str;
use ffi::*;
use fixed::types::I32F32; //FixedI32, consts,types::I32F32
use rust_core::encoder_core::SCALE;
use rust_core::serial_core::{Command, SerialParser, TermCmd};
// use heapless::String;
// use local_static::LocalStatic; //neded for write!
//                                // use heapless::String;
//                                // N is the max size of the string, including the null terminator.
//                                // We use 32, just like your raw_buf size.
//                                // static mut UART_BUFFER: Option<String<30>> = None;

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
        term(TermCmd::ClearScreen);
        term(TermCmd::Home);
        uart_put_str("\n\r-PSOC RS\r\n");
    }
    fn serial_event(&mut self, in32: u32) {
        let ch: u8 = (in32 & 0xFF) as u8;

        // single char (maybe non-printable), then hex
        // let one = [ch, 0];
        // unsafe { UART_UartPutString(one.as_ptr() as *const c_char) }
        match self.parser.parse_byte(ch) {
            Some(cmd) => match cmd {
                Command::Assign { var, value } => match var {
                    b'a' | b'A' => {
                        Xaxis.get_mut().encoder.g_a = value;
                        uart_printf(format_args!("\r\n New G_A: {:?}", value));
                    }
                    b'b' | b'B' => {
                        Xaxis.get_mut().encoder.g_b = value;
                    }
                    b'c' | b'C' => {
                        Xaxis.get_mut().encoder.g_c = value;
                    }
                    _ => {}
                },
                Command::ToggleDir => {
                    let dir = if Xaxis.get_mut().dir == MotorDirection::BWD {
                        uart_printf(format_args!("\n\rchange dir: FWD "));
                        MotorDirection::FWD
                    } else {
                        uart_printf(format_args!("\n\rchange dir: BWD "));

                        MotorDirection::BWD
                    };
                    Xaxis.get_mut().set_direction(dir);
                }
                Command::Kill => SYS.get_mut().next_state = System_State::KILL,
                Command::ToggleDebug => SYS.get_mut().print_dbg = !SYS.get().print_dbg,
                Command::StartMove => SYS.get_mut().next_state = System_State::START_MOVE,
                Command::StartSpeed => SYS.get_mut().next_state = System_State::START_SPD,
                Command::Stop => SYS.get_mut().next_state = System_State::STOPPING,
                Command::Reset => Xaxis.get_mut().encoder.zero(),
                Command::Unknown(unk_cmd) => uart_printf(format_args!(
                    "\n\rDBG: {} [{}]",
                    unk_cmd,
                    (unk_cmd as u8).escape_ascii()
                )),
            },
            None => {}
        }
    }
}

#[inline(always)]
pub fn uart_printf(args: core::fmt::Arguments<'_>) {
    UART.get_mut().write_fmt(args).unwrap();
}
enum DebugPrint {
    None,
    X_Motor,
    X_SPD,
    DEBUG,
}

fn term(cmd: TermCmd) {
    match cmd {
        TermCmd::ClearScreen => uart_printf(format_args!("\x1B[2J\x1B[H")),
        // TermCmd::ClearLine => uart_write(b"\x1B[K"),
        TermCmd::Home => uart_printf(format_args!("\x1B[H")),
    }
}

#[macro_export]
macro_rules! uart_println {
    ($($arg:tt)*) => {
        $crate::uart_printf(format_args!($($arg)*));
    };
}

#[unsafe(no_mangle)]
extern "C" fn RSUARTRX() {
    // RX_WAKER.signal(());
    unsafe {
        let ch: u32 = UART_UartGetChar();
        UART.get_mut().serial_event(ch);
        ClearInterrutpt_RX();
    }
}

pub fn uart_put_bytes(bytes: &[u8]) {
    // ensure null-terminated
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
pub fn uart_put_bytes_fast(bytes: &[u8]) {
    for &byte in bytes {
        unsafe { UART_SpiUartWriteTxData(byte as u32) }
    }
}
pub fn uart_put_bytes_unbuffered(bytes: &[u8]) {
    for &byte in bytes {
        unsafe { UART_SpiUartWriteTxData(byte as u32) }
    }
}
// Send raw bits (4 bytes, no conversion)
pub fn uart_send_i32f32_binary(value: I32F32) {
    let bits = value.to_bits();
    uart_put_tx((bits & 0xFF) as u32);
    uart_put_tx(((bits >> 8) & 0xFF) as u32);
    uart_put_tx(((bits >> 16) & 0xFF) as u32);
    uart_put_tx(((bits >> 24) & 0xFF) as u32);
}
pub fn uart_send_i32f32_hex(value: I32F32) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let bits = value.to_bits() as u32;

    for i in (0..8).rev() {
        let nibble = ((bits >> (i * 4)) & 0xF) as usize;
        uart_put_tx(HEX[nibble] as u32);
    }
}
// Helper: send u32 as decimal ASCII
pub fn uart_send_u32_decimal(mut value: u32) {
    if value == 0 {
        uart_put_tx(b'0' as u32);
        return;
    }

    let mut buf = [0u8; 10];
    let mut i = 0;

    while value > 0 {
        buf[i] = b'0' + (value % 10) as u8;
        value /= 10;
        i += 1;
    }

    // Send in reverse
    while i > 0 {
        i -= 1;
        uart_put_tx(buf[i] as u32);
    }
}
pub fn uart_send_i32_decimal(mut value: i32) {
    if value < 0 {
        uart_put_tx(b'-' as u32);
        value = -value;
    }
    uart_send_u32_decimal(value as u32);
}
pub fn uart_send_i64_decimal(mut value: i64) {
    if value < 0 {
        uart_put_tx(b'-' as u32);
        value = -value;
    }
    uart_send_u32_decimal((value >> 32) as u32);
    uart_send_u32_decimal((value & 0xFFFF_FFFF) as u32);
}
// Helper: send with leading zeros
fn uart_send_u32_decimal_padded(value: u32, width: u8) {
    let mut buf = [b'0'; 10];
    let mut temp = value;
    let mut i = width as usize;

    while i > 0 && temp > 0 {
        i -= 1;
        buf[i] = b'0' + (temp % 10) as u8;
        temp /= 10;
    }

    for j in 0..width as usize {
        uart_put_tx(buf[j] as u32);
    }
}
// Send scaled integer (multiply by 100, send as integer)
pub fn uart_send_i32f32_scaled(value: I32F32) {
    let scaled = (value * I32F32::from_num(100)).to_num::<i32>();
    uart_send_i32_decimal(scaled);
}
// Send multiple values in one packet
pub fn uart_send_i32f32_array(values: &[I32F32]) {
    for &value in values {
        uart_send_i32f32_binary(value);
    }
}

// Send u16 as 2 bytes
pub fn uart_send_u16(value: u16) {
    uart_put_tx((value & 0xFF) as u32);
    uart_put_tx_crlf(((value >> 8) & 0xFF) as u32);
}

// Send i32 (same as u32, just transmute)
pub fn uart_send_i32(value: i32) {
    uart_put_tx_crlf(value as u32);
}

// Send f32 as bytes
pub fn uart_send_f32(value: f32) {
    let bytes = value.to_le_bytes();
    for &byte in &bytes {
        uart_put_tx(byte as u32);
    }
}
pub fn uart_send_u32_hex(value: u32) {
    use core::fmt::Write;
    write!(UART.get_mut(), "0x{:08X}", value).ok();
}
pub fn uart_send_u16_hex(value: u16) {
    use core::fmt::Write;
    write!(UART.get_mut(), "0x{:04X}", value).ok();
}
