use crate::*;
use bitfield_struct::bitfield;
use ffi::*;

// register data from LED.h not captured by bindgen
// const LED_PS_ADDR: *const u32 = 0x40040104 as *const u32; // example address
// const LED_MASK: u32 = 0x40;
// const LED_SHIFT: u8 = 6;

// pub fn led_read() -> u8 {
//     unsafe { ((LED_PS_ADDR.read_volatile() & LED_MASK) >> LED_SHIFT) as u8 }
// }

// static mut LAST_TOGGLE: u32 = 0;
static mut TOGGLE_CNT: u16 = 0;

const BLINK_PERIOD: u16 = 4_8000; //2_400_000 //max

pub fn led_task() {
    // duration in ticks between blinks
    unsafe {
        TOGGLE_CNT += 1;
        if TOGGLE_CNT >= BLINK_PERIOD {
            TOGGLE_CNT = 0;
            LED_Write(!LED_Read());
        }
    }
}

const DEBOUNCE_PERIOD: u32 = 3000; //2_400_000 //max
const HELD_PERIOD: u32 = 12000; //2_400_000 //max
#[bitfield(u8)]
pub struct ButtonState {
    #[bits(1)]
    state: bool,
    #[bits(1)]
    pressed: bool,
    #[bits(1)]
    acknoledged: bool,
    #[bits(1)]
    held: bool,
    #[bits(4)]
    _reserved: u8,
}
pub struct DebouncedButton {
    read_fn: fn() -> bool, // GPIO read function
    btn: ButtonState,      // button compact state
    last_change: u32,      // last change time
}

impl DebouncedButton {
    pub fn new(read_fn: fn() -> bool) -> Self {
        let initial = read_fn();

        let mut btn_ = ButtonState::new();
        btn_.set_state(initial);
        DebouncedButton {
            read_fn,
            btn: btn_,
            last_change: 0,
        }
    }

    pub fn update(&mut self) -> bool {
        self.btn.set_state((self.read_fn)());

        // / if current != self.btn.last_state() { //useful for toggle switches
        //     // input changed, reset timing

        //     self.btn.set_last_state(current);
        //     uart_printf(format_args!("Button Change!\n\r"));
        // }

        // Check if enough time has passed to consider it stable
        if self.btn.state() {
            if self.last_change < u32::MAX {
                self.last_change += 1;
            }
            if self.btn.state() && self.last_change >= HELD_PERIOD {
                self.btn.set_held(true);
            }
            if self.last_change >= DEBOUNCE_PERIOD {
                // button pressed
                if !self.btn.acknoledged() {
                    // already acknowledged
                    self.btn.set_pressed(true);
                    self.btn.set_acknoledged(false);
                }
            }
        } else {
            self.last_change = 0;
            // button released
            self.btn.set_pressed(false);
            self.btn.set_held(false);
            self.btn.set_acknoledged(false);
        }

        self.btn.state()
    }

    pub fn is_pressed(&mut self) -> bool {
        if self.btn.pressed() {
            if !self.btn.acknoledged() {
                self.btn.set_acknoledged(true); // button read, reset acknowledged
                return true;
            }
        }
        false
    }
    pub fn is_held(&mut self) -> bool {
        self.btn.held()
    }
}
