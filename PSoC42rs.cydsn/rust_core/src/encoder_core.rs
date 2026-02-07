use crate::utils_core::{IirFilter, RingBuf};
use fixed::types::I16F16; //FixedI32, consts,types::I32F32

pub const COUNT_PER_REVI32: i32 = 1250;
// const COUNT_PER_REV: I16F16 = I16F16::from_bits(81_920_000);
pub const RAD_TO_COUNTS: I16F16 = I16F16::from_bits(330); // TWO_PI / COUNT_PER_REV
// IIR filter factors

// Deadbands
pub const ONE_I16F16: I16F16 = I16F16::from_bits(200);

pub const TS: I16F16 = I16F16::from_bits(1); // 1 ms
pub const OMEGA_ALPHA: I16F16 = I16F16::from_bits(19660);
pub const OMEGA_EPS: I16F16 = I16F16::from_bits(3276);
pub const ALPHA_ALPHA: I16F16 = I16F16::from_bits(13107);
pub const ALPHA_EPS: I16F16 = I16F16::from_bits(32768);

pub trait EncoderOps {
    fn init_hardware(&self);
    fn start_hardware(&self);
    #[cfg(not(target_os = "windows"))]
    fn write_counter(&self, value: u32);
    #[cfg(target_os = "windows")]
    fn write_counter(&mut self, value: u32);
    fn get_counter(&self) -> u32;
}

pub struct Encoder<T: EncoderOps> {
    pub counts: RingBuf<i32, 4>, // current raw counter value from hardware
    pub prev_enc_counts: i32,
    pub turns: i32, // number of overflows/underflows counted
    pub theta: I16F16,
    pub omega: I16F16,
    pub prev_omega: I16F16,
    pub alpha: I16F16,
    pub omega_filter: IirFilter,
    pub alpha_filter: IirFilter,
    pub delta_ticks: u32,
    ops: T,
}

impl<T: EncoderOps> Encoder<T> {
    pub fn new(ops: T) -> Self {
        ops.init_hardware();
        ops.start_hardware();
        Self {
            counts: RingBuf::new(0),
            turns: 0,
            theta: I16F16::from_bits(0),
            omega: I16F16::from_bits(0),
            alpha: I16F16::from_bits(0),
            omega_filter: IirFilter::new(I16F16::from_bits(11000)),
            alpha_filter: IirFilter::new(I16F16::from_bits(5000)),
            ops,
            prev_omega: I16F16::from_bits(0),
            prev_enc_counts: 0,
            delta_ticks: 0,
        }
    }
    #[cfg(not(target_arch = "arm"))]
    pub fn write_enc_counter(&mut self, value: u32) {
        self.ops.write_counter(value);
    }
    /// Returns the init of this [`Encoder`].
    pub fn init(&mut self) {
        self.counts = RingBuf::new(0);
        self.turns = 0;
        self.theta = I16F16::from_bits(0);
        self.omega = I16F16::from_bits(0);
        self.alpha = I16F16::from_bits(0);
        self.ops.init_hardware();
        self.ops.start_hardware();
        self.ops.write_counter(0);
        self.ops.get_counter();
        self.prev_enc_counts = 0;
        self.delta_ticks = 0;
    }

    /// Read the current encoder counter and update position
    /// measured 60us  max 96us
    pub fn update(&mut self, dt_ticks: u32) {
        // Pull the last three samples
        let (c0, c1) = match self.counts.get_last_two() {
            Some((a, b)) => (a, b),
            None => return,
        };

        let dt = I16F16::from_num(dt_ticks) / I16F16::from_num(24);
        // Prevent division by zero
        if dt <= I16F16::from_bits(0) {
            return;
        }
        // Compute deltas safely
        let dc1 = c0.wrapping_sub(c1) as i32;
        // let dc2 = c1.wrapping_sub(c2) as i32;
        // --- Theta (position) ---
        self.theta = I16F16::from_num(c0);

        // --- Omega (velocity) ---
        let mut omega_raw = I16F16::from_num(dc1) / dt;

        // Deadband small movements
        if omega_raw.abs() < OMEGA_EPS {
            omega_raw = I16F16::from_bits(0);
        }

        // Filtered omega
        let omega = OMEGA_ALPHA * self.omega + (ONE_I16F16 - OMEGA_ALPHA) * omega_raw;
        // self.omega = self.omega_filter.update(omega);
        self.omega = omega;
        // --- Alpha (acceleration) ---
        let mut alpha_raw = (self.omega - self.prev_omega) / dt;

        if alpha_raw.abs() < ALPHA_EPS {
            alpha_raw = I16F16::from_bits(0);
        }

        // // Light filter
        let alpha = ALPHA_ALPHA * self.alpha + (ONE_I16F16 - ALPHA_ALPHA) * alpha_raw;
        // // self.alpha = self.alpha_filter.update(alpha);
        self.alpha = alpha;
        self.prev_omega = self.omega;

        // if SYS.get_mut().print_dbg & 2 != 0 {
        //     uart_println!("Motor:{},{},{}\n\r", self.theta, self.omega, self.alpha);
        // }
    }

    /// Full encoder position as i32
    pub fn _read_position(&mut self) -> i32 {
        match self.counts.curr() {
            Some(v) => self.turns + (v as i32) - 0x8000, // assuming a 16-bit counter ,
            None => 0,
        }
    }

    /// Reads the current absolute position from the hardware counter.
    pub fn read_counter(&mut self) -> i32 {
        let count = self.ops.get_counter() as i32 - 0x8000;
        let dc1 = count.wrapping_sub(self.prev_enc_counts);
        if dc1 > (COUNT_PER_REVI32 >> 1) {
            self.turns -= 1;
        } else if dc1 < -(COUNT_PER_REVI32 >> 1) {
            self.turns += 1;
        }
        self.counts.push(count + self.turns * COUNT_PER_REVI32);
        self.prev_enc_counts = count;
        count
    }

    /// Resets the encoder counter to a specific value.
    pub fn _set_position(&mut self, value: u32) {
        self.ops.write_counter(value);
    }
}
