use core::prelude::rust_2024::*;

pub const SCALE: i64 = 65536;

fn mul(a: i64, b: i64) -> i64 {
    ((a as i128 * b as i128) / SCALE as i128) as i64
}

/// Linear ADRC (Active Disturbance Rejection Control) for embedded motion.
///
/// ## Position mode (second-order LESO)
/// - `z1`: estimated position (counts × SCALE)
/// - `z2`: estimated velocity (counts/sample × SCALE)
/// - `z3`: estimated total disturbance (counts/sample² × SCALE)
///
/// ## Speed mode (first-order LESO)
/// - `z1`: estimated velocity (counts/sample × SCALE)
/// - `z2`: estimated disturbance (counts/sample² × SCALE)
///
/// ### Tuning parameters
/// - `w0` — observer bandwidth in rad/s (Q16.16)
/// - `wc` — controller bandwidth in rad/s (Q16.16)
/// - `b0` — plant gain (Q16.16)
/// - `ts` — sample period in seconds (Q16.16 fraction, e.g. 200µs = 13 in SCALE)
///
/// All internal math uses Q16.16 fixed-point (SCALE = 65536),
/// matching `encoder_core`.
#[derive(Clone, Copy)]
pub struct Adrc {
    pub z1: i64,
    pub z2: i64,
    pub z3: i64,

    pub w0: u64,
    pub wc: u64,
    pub b0: u64,

    l1: i64,
    l2: i64,
    l3: i64,
    kp: i64,
    kd: i64,

    pub control: i64,
}

impl Adrc {
    pub const fn new() -> Self {
        Self {
            z1: 0,
            z2: 0,
            z3: 0,
            w0: 0,
            wc: 0,
            b0: SCALE as u64,
            l1: 0,
            l2: 0,
            l3: 0,
            kp: 0,
            kd: 0,
            control: 0,
        }
    }

    /// Tune for **position mode** (second-order LESO + LSEF).
    ///
    /// Observer: `l1=3·ω₀·Ts, l2=3·ω₀²·Ts, l3=ω₀³·Ts`
    /// Controller: `kp=ωc², kd=2·ωc`  (continuous gains)
    pub fn tune_position(&mut self, wo: u64, wc: u64, b0: u64, ts: u64) {
        self.w0 = wo;
        self.wc = wc;
        self.b0 = b0;
        let wo = mul(wo as i64, ts as i64);
        let wc = mul(wc as i64, ts as i64);
        self.l1 = 3i64.saturating_mul(wo);
        self.l2 = mul(3i64.saturating_mul(wo), wo);
        self.l3 = mul(mul(wo, wo), wo);
        self.kp = mul(wc, wc);
        self.kd = 2i64.saturating_mul(wc);
    }

    /// Tune for **speed mode** (first-order LESO + P control).
    ///
    /// Observer: `l1=2·ω₀·Ts, l2=ω₀²·Ts`
    /// Controller: `kp=ωc·Ts`  (discrete P gain)
    pub fn tune_speed(&mut self, wo: u64, wc: u64, b0: u64, ts: u64) {
        self.w0 = wo;
        self.wc = wc;
        self.b0 = b0;
        let wo = mul(wo as i64, ts as i64);
        let wc = mul(wc as i64, ts as i64);
        self.l1 = 2i64.saturating_mul(wo);
        self.l2 = mul(wo, wo);
        self.l3 = 0;
        self.kp = wc;
        self.kd = 0;
    }

    /// Position-mode update: second-order LESO + LSEF.
    ///
    /// `r`: reference position (counts × SCALE)
    /// `y`: measured position (counts × SCALE)
    pub fn update_position(&mut self, r: i64, y: i64) -> i64 {
        let e = self.z1.saturating_sub(y);

        self.z1 = self
            .z1
            .saturating_add(self.z2)
            .saturating_sub(mul(e, self.l1));

        self.z2 = self
            .z2
            .saturating_add(self.z3)
            .saturating_sub(mul(e, self.l2))
            .saturating_add(mul(self.control, self.b0 as i64));

        self.z3 = self.z3.saturating_sub(mul(e, self.l3));

        let u0 = mul(r.saturating_sub(self.z1), self.kp)
            .saturating_sub(mul(self.z2, self.kd));

        let num = u0.saturating_sub(self.z3) as i128;
        self.control =
            (num.saturating_mul(SCALE as i128) / (self.b0 as i128).max(1)) as i64;

        self.control
    }

    /// Speed-mode update: first-order LESO + P control.
    ///
    /// `r`: reference velocity (counts/sample × SCALE)
    /// `y`: measured velocity (counts/sample × SCALE)
    pub fn update_speed(&mut self, r: i64, y: i64) -> i64 {
        let e = self.z1.saturating_sub(y);

        self.z1 = self
            .z1
            .saturating_add(self.z2)
            .saturating_sub(mul(e, self.l1))
            .saturating_add(mul(self.control, self.b0 as i64));

        self.z2 = self.z2.saturating_sub(mul(e, self.l2));

        let u0 = mul(r.saturating_sub(self.z1), self.kp)
            .saturating_sub(mul(self.z2, self.kd));

        let num = u0.saturating_sub(self.z2) as i128;
        self.control =
            (num.saturating_mul(SCALE as i128) / (self.b0 as i128).max(1)) as i64;

        self.control
    }

    pub fn reset(&mut self) {
        self.z1 = 0;
        self.z2 = 0;
        self.z3 = 0;
        self.control = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 200 µs sample period in SCALE seconds
    const TS_200US: u64 = (SCALE as f64 * 200.0 / 1_000_000.0) as u64;

    fn setup_position() -> Adrc {
        let mut adrc = Adrc::new();
        adrc.tune_position(
            (500.0 * SCALE as f64) as u64,  // wo = 500 rad/s
            (250.0 * SCALE as f64) as u64,  // wc = 250 rad/s
            SCALE as u64,                    // b0 = 1.0
            TS_200US,
        );
        adrc
    }

    fn setup_speed() -> Adrc {
        let mut adrc = Adrc::new();
        adrc.tune_speed(
            (500.0 * SCALE as f64) as u64,
            (250.0 * SCALE as f64) as u64,
            SCALE as u64,
            TS_200US,
        );
        adrc
    }

    #[test]
    fn test_tune_position_gains_positive() {
        let adrc = setup_position();
        assert!(adrc.l1 > 0);
        assert!(adrc.l2 > 0);
        assert!(adrc.l3 > 0);
        assert!(adrc.kp > 0);
        assert!(adrc.kd > 0);
    }

    #[test]
    fn test_tune_speed_gains_positive() {
        let adrc = setup_speed();
        assert!(adrc.l1 > 0);
        assert!(adrc.l2 > 0);
        assert!(adrc.l3 == 0);
        assert!(adrc.kp > 0);
        assert!(adrc.kd == 0);
    }

    #[test]
    fn test_position_observer_converges() {
        let mut adrc = setup_position();
        let measured_y = 500 * SCALE;

        for _ in 0..500 {
            adrc.update_position(0, measured_y);
        }

        let err = (adrc.z1 - measured_y).abs() / SCALE;
        assert!(err < 50, "z1 failed to track y: err={}", err);
    }

    #[test]
    fn test_position_control_drives_plant() {
        let mut adrc = Adrc::new();
        adrc.tune_position(
            (500.0 * SCALE as f64) as u64,
            (250.0 * SCALE as f64) as u64,
            SCALE as u64,
            TS_200US,
        );

        let target = 500 * SCALE;
        let mut plant_pos = 0i64;
        let mut plant_vel = 0i64;

        for _ in 0..1000 {
            let u = adrc.update_position(target, plant_pos);
            plant_vel = plant_vel.saturating_add(u / SCALE);
            plant_vel = plant_vel.clamp(-100 * SCALE, 100 * SCALE);
            plant_pos = plant_pos.saturating_add(plant_vel);
        }

        // Plant moved toward target and z1 tracks position
        assert!(plant_pos > 0, "plant should move toward target");
        let track_err = (adrc.z1 - plant_pos).abs() / SCALE;
        assert!(track_err < 100, "observer should track plant");
    }

    #[test]
    fn test_speed_control_drives_plant() {
        let mut adrc = Adrc::new();
        adrc.tune_speed(
            (500.0 * SCALE as f64) as u64,
            (250.0 * SCALE as f64) as u64,
            SCALE as u64,
            TS_200US,
        );

        let mut plant_vel = 0i64;
        let target_vel = 1000 * SCALE;

        for _ in 0..200 {
            let u = adrc.update_speed(target_vel, plant_vel);
            plant_vel = plant_vel.saturating_add(u / SCALE);
        }

        assert!(plant_vel > 0, "plant should accelerate toward target");
        let track_err = (adrc.z1 - plant_vel).abs() / SCALE;
        assert!(track_err < 500, "observer should track plant: {}", track_err);
    }

    #[test]
    fn test_reset_clears_state() {
        let mut adrc = setup_position();
        adrc.update_position(100 * SCALE, 0);
        adrc.reset();
        assert_eq!(adrc.z1, 0);
        assert_eq!(adrc.z2, 0);
        assert_eq!(adrc.z3, 0);
        assert_eq!(adrc.control, 0);
    }

    #[test]
    fn test_tune_speed_gains_match_formula() {
        let ts = TS_200US;
        let wo_radps = (50.0 * SCALE as f64) as u64;
        let wc_radps = (20.0 * SCALE as f64) as u64;
        let b0 = SCALE as u64;
        let mut adrc = Adrc::new();
        adrc.tune_speed(wo_radps, wc_radps, b0, ts);

        let wo_s = mul(wo_radps as i64, ts as i64);
        let wc_s = mul(wc_radps as i64, ts as i64);

        assert_eq!(adrc.l1, 2 * wo_s);
        assert_eq!(adrc.kp, wc_s);
    }

    #[test]
    fn test_tune_position_gains_match_formula_approx() {
        let ts = TS_200US;
        let wo_radps = (10.0 * SCALE as f64) as u64;
        let wc_radps = (5.0 * SCALE as f64) as u64;
        let b0 = SCALE as u64;
        let mut adrc = Adrc::new();
        adrc.tune_position(wo_radps, wc_radps, b0, ts);

        let wo_s = mul(wo_radps as i64, ts as i64);
        let wc_s = mul(wc_radps as i64, ts as i64);

        assert_eq!(adrc.l1, 3 * wo_s);
        assert_eq!(adrc.kd, 2 * wc_s);
        let kp_expected = mul(wc_s, wc_s);
        assert!((adrc.kp - kp_expected).abs() <= 1);
    }

}
