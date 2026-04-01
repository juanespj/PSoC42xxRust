pub const COUNT_PER_REVI32: i32 = 1250;

pub const SCALE: i64 = 65536;
pub const DT_US: i64 = 200; // 0.1ms in micros
pub const DT_US2: i64 = DT_US * DT_US; // 0.6ms in micros

// Pre-calculate: dt/SCALE for multiplication
pub const DT_SCALED: i64 = (DT_US * SCALE) / 1_000_000; // dt in seconds * SCALE
// For 1/dt and 1/dt^2 - keep as constants to avoid division
pub const DT_INV_SCALED: i64 = (1_000_000 * SCALE) / (DT_US); // 1/dt * SCALE
pub const DT2_INV_SCALED: i64 = (1_000_000_000_000 * SCALE) / (DT_US2); // 1/dt^2 * SCALE

pub trait EncoderOps {
    fn init_hardware(&self);
    fn start_hardware(&self);
    #[cfg(target_arch = "arm")]
    fn write_counter(&self, value: u32);
    #[cfg(not(target_arch = "arm"))]
    fn write_counter(&mut self, value: u32);
    fn get_counter(&self) -> u32;
}

pub struct Encoder<T: EncoderOps> {
    // pub counts: RingBuf<i32, 4>, // current raw counter value from hardware
    pub counts: i64,
    pub prev_enc_counts: i32,
    pub turns: i32, // number of overflows/underflows counted

    pub pos: i64,
    pub vel: i64,
    pub prev_pos: i64,
    pub accel: i64,
    pub g_a: u64,
    pub g_b: u64,
    pub g_c: u64,

    pub smooth_vel: i64,
    pub smooth_accel: i64,
    ops: T,
}

impl<T: EncoderOps> Encoder<T> {
    pub fn new(ops: T) -> Self {
        ops.init_hardware();
        ops.start_hardware();
        Self {
            // counts: RingBuf::new(0),
            counts: 0,
            turns: 0,
            pos: 0,
            vel: 0,
            accel: 0,
            g_a: 21626, //0.25 * SCALE // >a20000,
            g_b: 655,   //0.05 //>b600,
            g_c: 14,    //0.005

            smooth_vel: 0, //0.6
            smooth_accel: 0,
            ops,
            prev_pos: 0,
            prev_enc_counts: 0,
        }
    }
    #[cfg(not(target_arch = "arm"))]
    pub fn write_enc_counter(&mut self, value: u32) {
        self.ops.write_counter(value);
    }
    /// Returns the init of this [`Encoder`].
    pub fn init(&mut self) {
        // self.counts = RingBuf::new(0);
        self.counts = 0;
        self.turns = 0;
        self.pos = 0;
        self.vel = 0;
        self.accel = 0;
        self.ops.init_hardware();
        self.ops.start_hardware();
        self.ops.write_counter(0);
        self.ops.get_counter();
        self.prev_enc_counts = 0;
    }

    pub fn update(&mut self) {
        // 1. Prediction (Physics Update)
        // Since dt is constant, we don't multiply by dt here to save CPU.
        // The units of vel/accel are effectively "per sample period".
        let p_pred = self
            .pos
            .saturating_add(self.vel)
            .saturating_add(self.accel >> 1);
        let v_pred = self.vel.saturating_add(self.accel);

        // 2. Innovation (Residual)
        let z_scaled = self.counts.saturating_mul(SCALE);
        let residual = z_scaled.saturating_sub(p_pred);

        // 3. Correction using i128 to prevent intermediate multiplication overflow
        let apply_gain =
            |res: i64, gain: i64| -> i64 { ((res as i128 * gain as i128) / SCALE as i128) as i64 };

        self.pos = p_pred.saturating_add(apply_gain(residual, self.g_a as i64));
        self.vel = v_pred.saturating_add(apply_gain(residual, self.g_b as i64));
        self.accel = self
            .accel
            .saturating_add(apply_gain(residual, self.g_c as i64));

        // 4. Physical Safety Clamps (Adjust these to your motor specs)
        // This prevents the filter from "exploding" if the encoder glitches
        const MAX_V: i64 = 100 * SCALE;
        const MAX_A: i64 = 100 * SCALE;
        self.vel = self.vel.clamp(-MAX_V, MAX_V);
        self.accel = self.accel.clamp(-MAX_A, MAX_A);

        self.smooth_vel = self
            .smooth_vel
            .saturating_add((self.vel.saturating_sub(self.smooth_vel)) >> 3);

        // 2. Efficient Acceleration Filter (EMA)
        // N=5 is a heavier filter to kill high-frequency spikes
        self.smooth_accel = self
            .smooth_accel
            .saturating_add((self.accel.saturating_sub(self.smooth_accel)) >> 5);
    }
    // Now these return human-readable values without overflow risk
    pub fn get_pos(&self) -> i32 {
        (self.pos / SCALE) as i32 // Result is in Counts/Sec
    }
    pub fn get_velocity(&self) -> i32 {
        (self.smooth_vel / 1000) as i32 // Result is in Counts/Sec
    }

    pub fn get_acceleration(&self) -> i32 {
        (self.smooth_accel) as i32 // Result is in Counts/Sec^2
    }
    /// Full encoder position as i32
    pub fn _read_position(&mut self) -> i64 {
        // match self.counts.curr() {
        //     Some(v) => self.turns + (v as i32) - 0x8000, // assuming a 16-bit counter ,
        //     None => 0,
        // }
        self.counts
    }
    pub fn zero(&mut self) {
        self.ops.write_counter(0x8000);
        self.counts = 0;
        self.prev_enc_counts = 0;
        self.turns = 0;
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
        self.counts = count as i64 + (self.turns * COUNT_PER_REVI32) as i64;
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
