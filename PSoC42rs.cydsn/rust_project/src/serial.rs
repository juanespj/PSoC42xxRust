// use crate::RX_WAKER;
use crate::SYS;
use crate::*;
use core::ffi::c_char;
use core::fmt::{self, Write};
use core::str;
use ffi::*;
// use heapless::String;
// use local_static::LocalStatic; //neded for write!
//                                // use heapless::String;
//                                // N is the max size of the string, including the null terminator.
//                                // We use 32, just like your raw_buf size.
//                                // static mut UART_BUFFER: Option<String<30>> = None;

struct UartWriter;

impl Write for UartWriter {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        uart_put_str(s);
        Ok(())
    }
}

pub fn uart_printf(args: core::fmt::Arguments<'_>) {
    let mut writer = UartWriter;
    writer.write_fmt(args).unwrap();
}
enum DebugPrint {
    None,
    X_Motor,
    X_SPD,
    DEBUG,
}
enum TermCmd {
    ClearScreen,
    // ClearLine,
    Home,
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
// const CLEAR_LINE: &[u8] = b"\x1B[K";

// const CURSOR_TOP_LEFT: &[u8] = b"\x1B[H";
// pub const UART_REGION_ON: &[u8; 6] = b"\x1B[?6h\0";
// pub const UART_REGION_OFF: &[u8; 6] = b"\x1B[?6l\0";
// pub const CLEARLINE: &[u8; 4] = b"\x1B[K\0";
// pub const CURSOR_HOME: &[u8; 4] = b"\x1B[H\0";
// pub const CLEAR_SCN: &[u8; 8] = b"\x1B[2J\x1B[H\0";
// pub const UART_CLEARSCRN: &[u8; 5] = b"\x1B[2J\0";
// pub const UART_CURSORHOME: &[u8; 4] = b"\x1B[H\0";
// pub const UART_VTSTATUS: &[u8; 5] = b"\x1B[c0\0";
// pub const UART_CLEARLINE: &[u8; 5] = b"\x1B[2K\0";
// pub const UART_CLEAR_EOL: &[u8; 4] = b"\x1B[K\0";
// pub const UART_MOVEUP: &[u8; 4] = b"\x1B[A\0";
// pub const UART_DHTOP: &[u8; 4] = b"\x1B#3\0";
// pub const UART_DHBOT: &[u8; 4] = b"\x1B#4\0";
// pub const UART_SWSH: &[u8; 4] = b"\x1B#5\0";
// pub const UART_DWSH: &[u8; 4] = b"\x1B#6\0";
// pub const UART_FBOLD: &[u8; 5] = b"\x1B[1m\0";
// pub const UART_FUNDERL: &[u8; 5] = b"\x1B[4m\0";
// pub const UART_FCLEAR: &[u8; 4] = b"\x1B[m\0";
// pub const UART_FRED: &[u8; 8] = b"\x1B[0;31m\0";
// pub const UART_FGRN: &[u8; 8] = b"\x1B[1;32m\0";

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

pub fn uart_put_bytes(bytes: &[u8]) {
    // ensure null-terminated
    let mut buf = [0u8; 64];
    let len = bytes.len().min(buf.len() - 1);
    buf[..len].copy_from_slice(&bytes[..len]);
    buf[len] = 0;
    unsafe { UART_UartPutString(buf.as_ptr() as *const c_char) }
}
pub fn uart_put_tx(d: u32) {
    unsafe { UART_SpiUartWriteTxData(d) }
}
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

// Send u32 as 4 bytes (little-endian)
pub fn uart_send_u32(value: u32) {
    uart_put_tx((value & 0xFF) as u32);
    uart_put_tx(((value >> 8) & 0xFF) as u32);
    uart_put_tx(((value >> 16) & 0xFF) as u32);
    uart_put_tx(((value >> 24) & 0xFF) as u32);
}

// Send u16 as 2 bytes
pub fn uart_send_u16(value: u16) {
    uart_put_tx((value & 0xFF) as u32);
    uart_put_tx(((value >> 8) & 0xFF) as u32);
}

// Send i32 (same as u32, just transmute)
pub fn uart_send_i32(value: i32) {
    uart_send_u32(value as u32);
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
    let mut writer = UartWriter;
    write!(writer, "0x{:08X}", value).ok();
}
pub fn uart_send_u16_hex(value: u16) {
    use core::fmt::Write;
    let mut writer = UartWriter;
    write!(writer, "0x{:04X}", value).ok();
}

fn serial_event(in32: u32) {
    let ch: u8 = (in32 & 0xFF) as u8;

    // single char (maybe non-printable), then hex
    // let one = [ch, 0];
    // unsafe { UART_UartPutString(one.as_ptr() as *const c_char) }

    match ch {
        b'd' | b'D' => {
            let dir = if Xaxis.get_mut().dir == MotorDirection::BWD {
                uart_printf(format_args!("\n\rchange dir: FWD "));
                MotorDirection::FWD
            } else {
                uart_printf(format_args!("\n\rchange dir: BWD "));

                MotorDirection::BWD
            };
            Xaxis.get_mut().set_direction(dir);
        }

        b'k' | b'K' => {
            // k
            SYS.get_mut().next_state = System_State::KILL;
        }
        b'p' => {
            SYS.get_mut().print_dbg = !SYS.get().print_dbg;
        }

        b'r' | b'R' => {
            // r
            SYS.get_mut().next_state = System_State::START_MOVE;
        }
        b's' | b'S' => {
            // r
            SYS.get_mut().next_state = System_State::START_SPD;
        }
        b't' | b'T' => {
            // r
            SYS.get_mut().next_state = System_State::STOPPING;
        }
        0 => { //null char, do nothing
        }
        other => {
            // {} calls the Display trait on the iterator, which prints the actual chars.

            uart_printf(format_args!(
                "\n\rDBG: {} [{}]",
                other,
                (other as u8).escape_ascii()
            ));
        }
    }
}
