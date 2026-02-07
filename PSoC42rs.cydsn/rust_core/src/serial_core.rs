// rust_core/src/serial.rs
use core::fmt;
use core::prelude::rust_2024::*; // for #[derive] support
use core::result::Result::Ok;
pub trait SerialWrite {
    fn write_bytes(&self, bytes: &[u8]);

    fn write_str(&self, s: &str) {
        self.write_bytes(s.as_bytes())
    }
    fn write_fmt(&self, args: fmt::Arguments<'_>);
}

pub struct FmtWriter<'a, T: SerialWrite>(pub &'a T);

impl<T: SerialWrite> fmt::Write for FmtWriter<'_, T> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.0.write_str(s);
        Ok(())
    }
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

pub fn term<W: SerialWrite>(w: &W, cmd: TermCmd) {
    match cmd {
        TermCmd::ClearScreen => w.write_str("\x1B[2J\x1B[H"),
        TermCmd::Home => w.write_str("\x1B[H"),
    }
}

// rust_core/src/commands.rs
#[derive(Debug, Clone, Copy)]
pub enum Command {
    ToggleDir,
    Kill,
    ToggleDebug,
    StartMove,
    StartSpeed,
    Stop,
    Unknown(u8),
}

pub fn parse_byte(b: u8) -> Option<Command> {
    match b {
        b'd' | b'D' => Some(Command::ToggleDir),
        b'k' | b'K' => Some(Command::Kill),
        b'p' => Some(Command::ToggleDebug),
        b'r' | b'R' => Some(Command::StartMove),
        b's' | b'S' => Some(Command::StartSpeed),
        b't' | b'T' => Some(Command::Stop),
        0 => None,
        other => Some(Command::Unknown(other)),
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
