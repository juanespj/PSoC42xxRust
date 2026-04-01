// rust_core/src/serial.rs
use core::fmt;
use core::prelude::rust_2024::*; // for #[derive] support
use core::result::Result::Ok;
// Core trait that abstracts the hardware interface
pub trait UartHardware {
    fn put_str(&self, s: &str);
    fn init(&self);
    fn clear_rx_buffer(&self);
}

// Generic UartIf that works with any hardware implementation
// pub struct UartIf<H: UartHardware> {
//     parser: SerialParser,
//     hardware: H,
// }

// impl<H: UartHardware> Write for UartIf<H> {
//     fn write_str(&mut self, s: &str) -> fmt::Result {
//         self.hardware.put_str(s);
//         Ok(())
//     }
// }

// impl<H: UartHardware> UartIf<H> {
//     pub fn new(hardware: H) -> Self {
//         Self {
//             parser: SerialParser::new(),
//             hardware,
//         }
//     }
//     pub fn UI_init(&mut self) {
//         self.hardware.init();
//         self.hardware.clear_rx_buffer();

//         // term(TermCmd::ClearScreen);
//         // term(TermCmd::Home);
//         self.hardware.put_str("\n\r-PSOC RS\r\n");
//     }
// }
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
    pub state: ParseState,
}

impl SerialParser {
    pub const fn new() -> Self {
        Self {
            state: ParseState::Idle,
        }
    }

    pub fn parse_byte(&mut self, b: u8) -> Option<Command> {
        match self.state {
            ParseState::Idle => match b {
                b'd' | b'D' => return Some(Command::ToggleDir),
                b'k' | b'K' => return Some(Command::Kill),
                b'p' => return Some(Command::ToggleDebug),
                b'r' | b'R' => return Some(Command::StartMove),
                b's' | b'S' => return Some(Command::StartSpeed),
                b't' | b'T' => return Some(Command::Stop),
                b'z' | b'Z' => return Some(Command::Reset),

                b'>' => {
                    self.state = ParseState::WaitVar;
                }

                0 => {}

                other => return Some(Command::Unknown(other)),
            },

            ParseState::WaitVar => {
                if b.is_ascii_alphabetic() {
                    self.state = ParseState::ReadNumber { var: b, value: 0 };
                } else {
                    self.state = ParseState::Idle;
                }
            }

            ParseState::ReadNumber { var, mut value } => match b {
                b'0'..=b'9' => {
                    value = value * 10 + (b - b'0') as u64;
                    self.state = ParseState::ReadNumber { var, value };
                }

                b',' => {
                    self.state = ParseState::Idle;
                    return Some(Command::Assign { var, value });
                }

                _ => {
                    self.state = ParseState::Idle;
                }
            },
        }

        None
    }
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
// pub fn term<W: SerialWrite>(w: &mut W, cmd: TermCmd) {
//     match cmd {
//         TermCmd::ClearScreen => {
//             w.write_str("\x1B[2J\x1B[H");
//         }
//         TermCmd::Home => {
//             w.write_str("\x1B[H");
//         }
//     }
// }
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
