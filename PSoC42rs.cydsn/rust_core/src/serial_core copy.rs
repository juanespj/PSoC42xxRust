// rust_core/src/serial.rs
#[cfg(target_arch = "arm")]
use core::fmt::{self, Write as FmtWrite};
use core::prelude::rust_2024::*; // for #[derive] support
use core::result::Result::Ok;
#[cfg(not(target_arch = "arm"))]
use std::io::{self, Write as IoWrite};
// Core trait that abstracts the hardware interface

pub trait UartHardware {
    fn put_str(&self, s: &str);
    fn init(&self);
    fn clear_rx_buffer(&self);

    fn put_tx(&self, byte: u32);
    fn put_crlf(&self, byte: u32);
    fn put_bytes(&self, bytes: &[u8]);
}

// Generic UartIf that works with any hardware implementation
pub struct UartIf<H: UartHardware> {
    pub parser: SerialParser,
    hardware: H,
}

#[cfg(target_arch = "arm")]

impl<H: UartHardware> FmtWrite for UartIf<H> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.hardware.put_str(s);
        Ok(())
    }
}

#[cfg(not(target_arch = "arm"))]
impl<H: UartHardware> IoWrite for UartIf<H> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        if let Ok(s) = core::str::from_utf8(buf) {
            self.hardware.put_str(s);
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

impl<H: UartHardware> UartIf<H> {
    #[inline(never)]
    pub fn new(hardware: H) -> Self {
        Self {
            parser: SerialParser::new(),
            hardware,
        }
    }
    #[inline(never)]
    pub fn ui_init(&mut self) {
        self.hardware.init();
        self.hardware.clear_rx_buffer();

        self.term(TermCmd::ClearScreen);
        self.term(TermCmd::Home);
        self.hardware.put_str("\n\r-PSOC RS\r\n");
    }
    pub fn parse_byte(&mut self, b: u8) {
        parse_byte(&mut self.parser, b);
    }
    #[inline(always)]
    pub fn put_tx(&mut self, d: u32) {
        self.hardware.put_tx(d);
    }

    #[inline(always)]
    pub fn put_tx_crlf(&mut self, d: u32) {
        self.hardware.put_crlf(d);
    }

    #[inline(always)]
    pub fn put_str(&mut self, s: &str) {
        self.hardware.put_str(s);
    }

    pub fn put_bytes(&mut self, bytes: &[u8]) {
        self.hardware.put_bytes(bytes);
    }

    pub fn put_bytes_fast(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.hardware.put_tx(byte as u32);
        }
    }

    pub fn put_bytes_unbuffered(&mut self, bytes: &[u8]) {
        for &byte in bytes {
            self.hardware.put_tx(byte as u32);
        }
    }

    pub fn term(&mut self, cmd: TermCmd) {
        match cmd {
            TermCmd::ClearScreen => {
                self.put_str("\x1B[2J\x1B[H");
            }
            TermCmd::Home => {
                self.put_str("\x1B[H");
            }
        }
    }
    // ============= BINARY SEND METHODS =============

    // pub fn send_i32f32_binary(&mut self, value: I32F32) {
    //     let bits = value.to_bits();
    //     self.put_tx((bits & 0xFF) as u32);
    //     self.put_tx(((bits >> 8) & 0xFF) as u32);
    //     self.put_tx(((bits >> 16) & 0xFF) as u32);
    //     self.put_tx(((bits >> 24) & 0xFF) as u32);
    // }

    // pub fn send_i32f32_hex(&mut self, value: I32F32) {
    //     const HEX: &[u8; 16] = b"0123456789ABCDEF";
    //     let bits = value.to_bits() as u32;

    //     for i in (0..8).rev() {
    //         let nibble = ((bits >> (i * 4)) & 0xF) as usize;
    //         self.put_tx(HEX[nibble] as u32);
    //     }
    // }

    pub fn send_u32_dec(&mut self, mut value: u32) {
        if value == 0 {
            self.put_tx(b'0' as u32);
            return;
        }

        let mut buf = [0u8; 10];
        let mut i = 0;

        while value > 0 {
            buf[i] = b'0' + (value % 10) as u8;
            value /= 10;
            i += 1;
        }

        while i > 0 {
            i -= 1;
            self.put_tx(buf[i] as u32);
        }
    }

    pub fn send_i32_dec(&mut self, mut value: i32) {
        if value < 0 {
            self.put_tx(b'-' as u32);
            value = -value;
        }
        self.send_u32_dec(value as u32);
    }

    pub fn send_i64_dec(&mut self, mut value: i64) {
        if value < 0 {
            self.put_tx(b'-' as u32);
            value = -value;
        }
        // Note: This needs fixing - can't just send high/low parts separately
        self.send_u32_dec((value >> 32) as u32);
        self.send_u32_dec((value & 0xFFFF_FFFF) as u32);
    }

    // fn send_u32_dec_padded(&mut self, value: u32, width: u8) {
    //     let mut buf = [b'0'; 10];
    //     let mut temp = value;
    //     let mut i = width as usize;

    //     while i > 0 && temp > 0 {
    //         i -= 1;
    //         buf[i] = b'0' + (temp % 10) as u8;
    //         temp /= 10;
    //     }

    //     for j in 0..width as usize {
    //         self.put_tx(buf[j] as u32);
    //     }
    // }

    // pub fn send_i32f32_scaled(&mut self, value: I32F32) {
    //     let scaled = (value * I32F32::from_num(100)).to_num::<i32>();
    //     self.send_i32_dec(scaled);
    // }

    // pub fn send_i32f32_array(&mut self, values: &[I32F32]) {
    //     for &value in values {
    //         self.send_i32f32_binary(value);
    //     }
    // }

    pub fn send_u16(&mut self, value: u16) {
        self.put_tx((value & 0xFF) as u32);
        self.put_tx_crlf(((value >> 8) & 0xFF) as u32);
    }

    pub fn send_i32(&mut self, value: i32) {
        self.put_tx_crlf(value as u32);
    }

    pub fn send_f32(&mut self, value: f32) {
        let bytes = value.to_le_bytes();
        for &byte in &bytes {
            self.put_tx(byte as u32);
        }
    }
    #[inline(always)]
    pub fn printf(&mut self, args: core::fmt::Arguments<'_>) {
        use core::fmt::Write;
        self.write_fmt(args).unwrap();
    }
    // pub fn send_u32_hex(&mut self, value: u32) {
    //     use core::fmt::Write;
    //     write!(self, "0x{:08X}", value).ok();
    // }

    // pub fn send_u16_hex(&mut self, value: u16) {
    //     use core::fmt::Write;
    //     write!(self, "0x{:04X}", value).ok();
    // }
}
#[derive(Debug, Clone, Copy)]
pub enum Command {
    ToggleDir,
    Kill,
    ToggleDebug,
    StartMove,
    StartSpeed,
    Stop,
    Reset,
    Assign { var: u8, value: u64 },
    Unknown(u8),
}

#[derive(Copy, Clone)]
enum ParseState {
    Idle,
    WaitVar,
    ReadNumber { var: u8, value: u64 },
}

pub struct SerialParser {
    state: ParseState,
}

impl SerialParser {
    pub const fn new() -> Self {
        Self {
            state: ParseState::Idle,
        }
    }
}

pub fn parse_byte(parser: &mut SerialParser, b: u8) -> Option<Command> {
    match parser.state {
        ParseState::Idle => match b {
            b'd' | b'D' => return Some(Command::ToggleDir),
            b'k' | b'K' => return Some(Command::Kill),
            b'p' => return Some(Command::ToggleDebug),
            b'r' | b'R' => return Some(Command::StartMove),
            b's' | b'S' => return Some(Command::StartSpeed),
            b't' | b'T' => return Some(Command::Stop),
            b'z' | b'Z' => return Some(Command::Reset),

            b'>' => {
                parser.state = ParseState::WaitVar;
            }

            0 => {}

            other => return Some(Command::Unknown(other)),
        },

        ParseState::WaitVar => {
            if b.is_ascii_alphabetic() {
                parser.state = ParseState::ReadNumber { var: b, value: 0 };
            } else {
                parser.state = ParseState::Idle;
            }
        }

        ParseState::ReadNumber { var, mut value } => match b {
            b'0'..=b'9' => {
                value = value * 10 + (b - b'0') as u64;
                parser.state = ParseState::ReadNumber { var, value };
            }

            b',' => {
                parser.state = ParseState::Idle;
                return Some(Command::Assign { var, value });
            }

            _ => {
                parser.state = ParseState::Idle;
            }
        },
    }

    None
}
#[macro_export]
macro_rules! uart_println {
    ($writer:expr, $($arg:tt)*) => {{
        use core::fmt::Write;
        let mut w = $crate::serial::FmtWriter($writer);
        let _ = writeln!(w, $($arg)*);
    }};
}

pub enum TermCmd {
    ClearScreen,
    Home,
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
