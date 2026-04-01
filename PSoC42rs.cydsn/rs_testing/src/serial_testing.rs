// ============= PC IMPLEMENTATION =============
use std::io::{self, Write as IoWrite};

pub struct PcUartHw;

impl UartHardware for PcUartHw {
    fn put_str(&self, s: &str) {
        print!("{}", s);
        io::stdout().flush().ok();
    }

    fn init(&self) {
        println!("[PC UART] Initialized");
    }

    fn clear_rx_buffer(&self) {
        // No-op
    }

    fn put_tx(&self, d: u32) {
        print!("{}", (d & 0xFF) as u8 as char);
        io::stdout().flush().ok();
    }

    fn put_crlf(&self, d: u32) {
        self.put_tx(d);
        print!("\r\n");
        io::stdout().flush().ok();
    }

    fn put_bytes(&self, bytes: &[u8]) {
        io::stdout().write_all(bytes).ok();
        io::stdout().flush().ok();
    }
}
