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

enum TermCmd {
    ClearScreen,
    // ClearLine,
    Home,
}

fn term(cmd: TermCmd) {
    match cmd {
        TermCmd::ClearScreen => uart_write(b"\x1B[2J\x1B[H"),
        // TermCmd::ClearLine => uart_write(b"\x1B[K"),
        TermCmd::Home => uart_write(b"\x1B[H"),
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
    // term(TermCmd::ClearScreen);
    //  term(TermCmd::Home);
    uart_put_str("\n\r-PSOC RS\r\n");
}
#[no_mangle]
extern "C" fn RSUARTRX() {
    // RX_WAKER.signal(());
    unsafe {
        let ch: u32 = UART_UartGetChar();

        //   UART_SpiUartWriteTxData(ch);
        // data available
        //  let ch_u8 = ch as u8;
        //   let buffer = [ch_u8, 0]; // null-terminated
        //   UART_UartPutString(buffer.as_ptr() as *const c_char);
        //   serialEvent(ch); // verify the character that was read by the UART
        serial_event(ch);
        ClearInterrutpt_RX();
    }
}
fn uart_write(bytes: &[u8]) {
    // null-terminate automatically
    let mut buf = [0u8; 20];
    let n = bytes.len().min(buf.len() - 1);
    buf[..n].copy_from_slice(bytes);
    buf[n] = 0;

    unsafe { UART_UartPutString(buf.as_ptr() as *const core::ffi::c_char) }
}

fn uart_put_hex_byte(b: u8) {
    const HEX: &[u8; 16] = b"0123456789ABCDEF";
    let hi = HEX[(b >> 4) as usize];
    let lo = HEX[(b & 0xF) as usize];
    let buf = [b'0', b'x', hi, lo, b'\n', b'\r', 0];
    unsafe { UART_UartPutString(buf.as_ptr() as *const c_char) }
}

pub fn uart_put_bytes(bytes: &[u8]) {
    // ensure null-terminated
    let mut buf = [0u8; 64];
    let len = bytes.len().min(buf.len() - 1);
    buf[..len].copy_from_slice(&bytes[..len]);
    buf[len] = 0;
    unsafe { UART_UartPutString(buf.as_ptr() as *const c_char) }
}

pub fn uart_put_str(s: &str) {
    uart_put_bytes(s.as_bytes())
}

fn serial_event(in32: u32) {
    let ch: u8 = (in32 & 0xFF) as u8;

    // single char (maybe non-printable), then hex
    // let one = [ch, 0];
    // unsafe { UART_UartPutString(one.as_ptr() as *const c_char) }

    match ch {
        b'p' => {
            uart_put_str("\n\r-Print\n\r");
            // toggle section
            // memset(input_arr,0,sizeof)
            // INPUT_ARR.lock(|arr| arr.fill(0)); // example
        }

        b'r' => {
            // r
            SYS.get_mut().next_state = System_State::START_MOVE;
        }
        b'k' => {
            // k
            SYS.get_mut().next_state = System_State::KILL;
        }

        other => {
            let tmp = [other, 0];
            unsafe {
                uart_put_str("\n\rDBG: read byte: ");
                uart_put_hex_byte(ch);
                UART_SpiUartWriteTxData(in32);
                UART_UartPutString(tmp.as_ptr() as *const c_char)
            }
        }
    }
}

// #[embassy_executor::task]
// pub async fn uart_tx_task(mut tx: UarteTx<'static, embassy_nrf::peripherals::UARTE0>) {
//     info!("UART TX TASK Running!");
//     loop {
//         let msg = CHANPRINT.receive().await;
//         unwrap!(tx.write(msg.as_bytes()).await);
//     }
// }
