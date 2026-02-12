use core::ops::Mul;

use crate::utils_core::{IirFilter, RingBuf};
pub const COUNT_PER_REVI32: i32 = 1250;

pub const SCALE: i64 = 65536;
pub const DT_US: i64 = 600; // 0.6ms in micros
pub const DT_US2 :i64= DT_US*DT_US; // 0.6ms in micros

// Gains (Alpha, Beta, Gamma) scaled by 2^16
// These are tuned for a Tracking Index of ~0.5
pub const GA: u64 = 32768; // 0.5 * SCALE
pub const GB: u64 = 13107; // 0.2 * SCALE
pub const GC: u64 = 3276; // 0.05 * SCALE
// Pre-calculate: dt/SCALE for multiplication
pub const DT_SCALED: i64 = (DT_US * SCALE) / 1_000_000; // dt in seconds * SCALE
// For 1/dt and 1/dt^2 - keep as constants to avoid division
pub const DT_INV_SCALED: i64 = (1_000_000 * SCALE) / (DT_US); // 1/dt * SCALE
pub const DT2_INV_SCALED: i64 = (1_000_000_000_000 * SCALE ) / (DT_US2); // 1/dt^2 * SCALE
#[cfg(feature = "embedded")]
pub mod config {

    use crate::encoder_core::SCALE;

    pub fn gain_a() -> u64 {
        32768 //0.5 *scale
    }
    pub fn gain_b() -> u64 {
        13107 //0.2 in fixed point
    }
    pub fn gain_c() -> u64 {
        3276 //0.05 in fixed point
    }
}

#[cfg(not(feature = "embedded"))] // PC target
pub mod config {
    use core::cell::Cell;

    // We need a wrapper that is Sync so it can live in a static
    pub struct Tunable {
        value: Cell<u64>,
    }

    // This is "unsafe" in theory, but on a single-core MCU like PSoC4,
    // it is practically safe as long as you aren't writing in an ISR
    // and reading in the main loop simultaneously.
    unsafe impl Sync for Tunable {}

    impl Tunable {
        pub const fn new(default_bits: u64) -> Self {
            Self {
                value: Cell::new(default_bits as u64),
            }
        }

        #[inline(always)]
        pub fn get(&self) -> u64 {
            self.value.get() // Returns a copy (I32F32 is Copy)
        }

        #[inline(always)]
        pub fn set(&self, v: u64) {
            self.value.set(v);
        }
    }
    // Now define your statics with their defaults directly
    pub static GAIN_A: Tunable = Tunable::new(0);
    pub static GAIN_B: Tunable = Tunable::new(0);
    pub static GAIN_C: Tunable = Tunable::new(0);
    // pub static ALPHA_EPS: Tunable = Tunable::new(32768);

    // Your existing helper functions will now work perfectly:
    pub fn gain_a() -> u64 {
        GAIN_A.get()
    }

    pub fn set_gain_a(v: u64) {
        GAIN_A.set(v);
    }

    pub fn gain_b() -> u64 {
        GAIN_B.get()
    }

    pub fn set_gain_b(v: u64) {
        GAIN_B.set(v);
    }
    pub fn gain_c() -> u64 {
        GAIN_C.get()
    }

    pub fn set_gain_c(v: u64) {
        GAIN_C.set(v);
    }
}
use config::*;
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

    pub theta: i64,
    pub omega: i64,
    pub prev_omega: i64,
    pub alpha: i64,
    pub omega_filter: IirFilter,
    pub alpha_filter: IirFilter,
    ops: T,
}

impl<T: EncoderOps> Encoder<T> {
    pub fn new(ops: T) -> Self {
        ops.init_hardware();
        ops.start_hardware();
        Self {
            counts: RingBuf::new(0),
            turns: 0,
            theta: 0,
            omega: 0,
            alpha: 0,
            omega_filter: IirFilter::new(0x99999A00), //0.6
            alpha_filter: IirFilter::new(0x100000000),
            ops,
            prev_omega: 0,
            prev_enc_counts: 0,
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
        self.theta = 0;
        self.omega = 0;
        self.alpha = 0;
        self.ops.init_hardware();
        self.ops.start_hardware();
        self.ops.write_counter(0);
        self.ops.get_counter();
        self.prev_enc_counts = 0;
    }

    pub fn update(&mut self) {
        // --- CONSTANTS (Tuned for 600us) ---

        // 1. Prediction (Physics)
        // pos = pos + vel*dt + 0.5*accel*dt^2
        // Using integer math, we keep units in "Counts" and "Counts/ms"
        let p_pred = self.theta + (self.omega * DT_US / 1000);
        let v_pred = self.omega + (self.alpha * DT_US / 1000);

        // 2. Innovation (Residual)
        // Shift raw count up to match our scaled internal state
        let z_scaled = (self.counts.curr().unwrap() as i64) * SCALE;
        let residual = z_scaled - p_pred;

        // 3. Correction
        // We use saturating_add to ensure no hardware interrupts/panics
        // if the encoder skips or glitches.
        self.theta = p_pred.saturating_add((gain_a() as i64* residual) / SCALE);
        self.omega = v_pred.saturating_add((gain_b()as i64 * residual) / SCALE);
        self.alpha = self.alpha.saturating_add((gain_c() as i64* residual) / SCALE);
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

// Read the current encoder counter and update position
// measured 60us  max 96us
// pub fn _update_z(&mut self, dt_ticks: u32) {
//     // Pull the last three samples
//     let (c0, c1) = match self.counts.get_last_two() {
//         Some((a, b)) => (a, b),
//         None => return,
//     };

//     let dt = I32F32::from_num(dt_ticks) / I32F32::from_num(24);
//     // Prevent division by zero
//     if dt <= 0 {
//         return;
//     }
//     // Compute deltas safely
//     // let dc2 = c1.wrapping_sub(c2) as i32;
//     // --- Theta (position) ---
//     self.theta = I32F32::from_num(c0);
//     let dc1 = c0.wrapping_sub(c1) as i32;

//     // --- Omega (velocity) ---
//     let omega_raw = I32F32::from_num(dc1) / dt;

//     // Filtered omega
//     let omega = self.omega * omega_alpha() + (ONE_I32F32 - omega_alpha()) * omega_raw;
//     self.omega = self.omega_filter.update(omega);
//     // self.omega = omega;
//     // --- Alpha (acceleration) ---
//     let alpha_raw = (self.omega - self.prev_omega) / dt;

//     // Light filter
//     let alpha = self.alpha * alpha_alpha() + (ONE_I32F32 - alpha_alpha()) * alpha_raw;
//     self.alpha = self.alpha_filter.update(alpha);
//     // self.alpha = alpha;
//     self.prev_omega = self.omega;
// }

// State variables stored in your struct
// self.pos: I32F32
// self.vel: I32F32
// self.accel: I32F32
