use crate::ffi::*;
use rs_core::ui::{Led, LedIo};

pub use rs_core::ui::DebouncedButton;

const BLINK_PERIOD: u32 = 1_000;

/// LED GPIO binding for this board.
pub struct XLed;
impl LedIo for XLed {
    fn write(&self, value: u8) {
        unsafe { LED_Write(value) };
    }
    fn read(&self) -> u8 {
        unsafe { LED_Read() }
    }
}

/// Board LED blinker. Thin wrapper around `rs_core::ui::Led` so the
/// existing `LED_CTRL::new()` / `led_task()` call sites stay unchanged.
pub struct LED_CTRL(Led<XLed>);
impl LED_CTRL {
    pub fn new() -> Self {
        LED_CTRL(Led::new(XLed, BLINK_PERIOD))
    }
    #[inline(always)]
    pub fn led_task(&mut self) {
        self.0.led_task();
    }
}
