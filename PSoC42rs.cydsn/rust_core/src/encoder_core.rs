use crate::utils_core::{IirFilter, RingBuf};
use fixed::types::I32F32; //FixedI32, consts,types::I32F32
use local_static::LocalStatic;
pub const COUNT_PER_REVI32: i32 = 1250;
pub const RAD_TO_COUNTS: I32F32 = I32F32::from_bits(330); // TWO_PI / COUNT_PER_REV
pub const ONE_I32F32: I32F32 = I32F32::from_bits(200);
pub const TS: I32F32 = I32F32::from_bits(1);
pub const ZERO_FIVE: I32F32 = I32F32::from_bits(0x3FE0000000000000);
pub const ONE_K_I32F32: I32F32 = I32F32::from_bits(65536000);
pub const FIVEH_K_I32F32: I32F32 = I32F32::from_bits(2147483648000000);
const DT_SCALE: I32F32 = I32F32::from_bits(0x0000_AAAB); // ~1/24 in fixed point
// Use ticks directly in prediction (scale back at the end)
const SCALE_24: I32F32 = I32F32::from_bits(178956971); //1/24 -> 0.0416666667

#[cfg(feature = "embedded")]
pub mod config {

    use fixed::types::I32F32;

    #[inline(always)]
    pub fn omega_alpha() -> I32F32 {
        I32F32::from_bits(0x80000000) //0.5
    }
    #[inline(always)]
    pub fn alpha_alpha() -> I32F32 {
        I32F32::from_bits(0xCF5C2900) //0.81
    }
    // #[inline(always)]
    // pub fn omega_eps() -> I32F32 {
    //     I32F32::from_bits(3276)
    // }

    // #[inline(always)]
    // pub fn alpha_eps() -> I32F32 {
    //     I32F32::from_bits(0x80000000) //0.5
    // }
    pub fn gain_a() -> I32F32 {
        I32F32::from_bits(0x947AE200) //0.5
    }

    pub fn gain_b() -> I32F32 {
        I32F32::from_bits(0x3AE147C0) //0.5
    }

    pub fn gain_c() -> I32F32 {
        I32F32::from_bits(0x0CCCCCD0) //0.5
    }
}

#[cfg(not(feature = "embedded"))] // PC target
pub mod config {
    use core::cell::Cell;
    use fixed::types::I32F32;

    // We need a wrapper that is Sync so it can live in a static
    pub struct Tunable {
        value: Cell<I32F32>,
    }

    // This is "unsafe" in theory, but on a single-core MCU like PSoC4,
    // it is practically safe as long as you aren't writing in an ISR
    // and reading in the main loop simultaneously.
    unsafe impl Sync for Tunable {}

    impl Tunable {
        pub const fn new(default_bits: i64) -> Self {
            Self {
                value: Cell::new(I32F32::from_bits(default_bits)),
            }
        }

        #[inline(always)]
        pub fn get(&self) -> I32F32 {
            self.value.get() // Returns a copy (I32F32 is Copy)
        }

        #[inline(always)]
        pub fn set(&self, v: I32F32) {
            self.value.set(v);
        }
    }
    // Now define your statics with their defaults directly
    pub static GAIN_A: Tunable = Tunable::new(0x947AE200);
    pub static GAIN_B: Tunable = Tunable::new(0x3AE147C0);
    pub static GAIN_C: Tunable = Tunable::new(0x0CCCCCD0);
    // pub static ALPHA_EPS: Tunable = Tunable::new(32768);

    // Your existing helper functions will now work perfectly:
    pub fn gain_a() -> I32F32 {
        GAIN_A.get()
    }

    pub fn set_gain_a(v: I32F32) {
        GAIN_A.set(v);
    }

    pub fn gain_b() -> I32F32 {
        GAIN_B.get()
    }

    pub fn set_gain_b(v: I32F32) {
        GAIN_B.set(v);
    }
    pub fn gain_c() -> I32F32 {
        GAIN_C.get()
    }

    pub fn set_gain_c(v: I32F32) {
        GAIN_C.set(v);
    }
    // #[inline(always)]
    // pub fn omega_eps() -> I32F32 {
    //     OMEGA_EPS.get()
    // }

    // pub fn set_omega_eps(v: I32F32) {
    //     OMEGA_EPS.set(v);
    // }

    // #[inline(always)]
    // pub fn alpha_alpha() -> I32F32 {
    //     ALPHA_ALPHA.get()
    // }

    // pub fn set_alpha_alpha(v: I32F32) {
    //     ALPHA_ALPHA.set(v);
    // }

    // #[inline(always)]
    // pub fn alpha_eps() -> I32F32 {
    //     ALPHA_EPS.get()
    // }

    // pub fn set_alpha_eps(v: I32F32) {
    //     ALPHA_EPS.set(v);
    // }
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
    pub pos: I32F32,
    pub vel: I32F32,
    pub accel: I32F32,

    pub theta: I32F32,
    pub omega: I32F32,
    pub prev_omega: I32F32,
    pub alpha: I32F32,
    pub omega_filter: IirFilter<I32F32>,
    pub alpha_filter: IirFilter<I32F32>,
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
            theta: I32F32::from_bits(0),
            omega: I32F32::from_bits(0),
            alpha: I32F32::from_bits(0),
            omega_filter: IirFilter::new(I32F32::from_bits(0x99999A00)), //0.6
            alpha_filter: IirFilter::new(I32F32::from_bits(0x100000000)),
            ops,
            prev_omega: I32F32::from_bits(0),
            prev_enc_counts: 0,
            delta_ticks: 0,
            pos: I32F32::from_bits(0),
            vel: I32F32::from_bits(0),
            accel: I32F32::from_bits(0),
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
        self.theta = I32F32::from_bits(0);
        self.omega = I32F32::from_bits(0);
        self.alpha = I32F32::from_bits(0);
        self.ops.init_hardware();
        self.ops.start_hardware();
        self.ops.write_counter(0);
        self.ops.get_counter();
        self.prev_enc_counts = 0;
        self.delta_ticks = 0;
        self.pos = I32F32::from_bits(0);
        self.accel = I32F32::from_bits(0);
        self.vel = I32F32::from_bits(0);
    }

    /// Read the current encoder counter and update position
    /// measured 60us  max 96us
    // pub fn _update_z(&mut self, dt_ticks: u32) {
    //     // Pull the last three samples
    //     let (c0, c1) = match self.counts.get_last_two() {
    //         Some((a, b)) => (a, b),
    //         None => return,
    //     };

    //     let dt = I32F32::from_num(dt_ticks) / I32F32::from_num(24);
    //     // Prevent division by zero
    //     if dt <= I32F32::from_bits(0) {
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

    pub fn update(&mut self, dt_ticks: u32) {
        // Pre-multiply by reciprocal instead of dividing
        let dt_ticks_i32 = I32F32::from_num(dt_ticks);
        if dt_ticks < 96 {
            // 96/24 = 4, equivalent to your check
            return;
        }

        // 1. Prediction
        let dt_24_2 = I32F32::from_num(dt_ticks >> 1).saturating_div(SCALE_24);
        let p_pred = self
            .pos
            .saturating_add(
                self.vel
                    .saturating_mul(dt_ticks_i32.saturating_div(SCALE_24)),
            )
            .saturating_add(self.accel.saturating_mul(dt_24_2).saturating_mul(dt_24_2));
        let v_pred = self.vel.saturating_add(
            self.accel
                .saturating_mul(dt_ticks_i32.saturating_div(SCALE_24)),
        );

        // 2. Innovation
        let residual = I32F32::from_num(self.counts.curr().unwrap()).saturating_sub(p_pred);

        // 3. Correction - precompute reciprocals
        // Use pre-scaled gains
        self.pos = p_pred.saturating_add(gain_a().saturating_mul(residual));
        self.vel =
            v_pred.saturating_add(gain_b().saturating_mul(residual.saturating_div(dt_ticks_i32)));
        self.accel = self.accel.saturating_add(
            gain_c().saturating_mul(residual.saturating_div(dt_24_2.saturating_mul(dt_24_2))),
        );

        self.theta = self.pos;
        self.omega = self.vel;
        self.alpha = self.accel;
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
