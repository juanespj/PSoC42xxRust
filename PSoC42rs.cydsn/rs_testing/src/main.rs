// Use the #[path] attribute to point to the specific .rs files
// 1. Direct paths to sibling crate source files
use fixed::types::I16F16;
use rust_core::encoder_core::{Encoder, EncoderOps};
use rust_core::utils_core::{IirFilter, RingBuf};
fn main() {
    println!("Successfully accessing PSoC source files from Host!");
    // Example: let enc = Encoder::new();
}

#[cfg(test)]
mod Encoder_tests {

    use super::*;
    use core::f32;
    use fixed::{FixedI32, consts, types::I16F16, types::I32F32};
    use gnuplot::{AxesCommon, Caption, Color, ColorIndex, Figure};
    use std::{iter, ops::Range};

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
    pub struct MockEncoder {
        counter: u32,
    }
    impl EncoderOps for MockEncoder {
        fn init_hardware(&self) {
            unsafe {
                println!("MochInit Encoder");
            }
        }
        fn start_hardware(&self) {}
        fn write_counter(&mut self, value: u32) {
            self.counter = value;
        }
        #[inline(always)]
        fn get_counter(&self) -> u32 {
            self.counter
        }
    }
    //     counts
    //   ^
    //   |        ┌───────┐
    //   |       /         \
    //   |______/           \______
    //            time →
    fn ramp_hold_ramp(
        t_ms: u32,
        up_ms: u32,
        hold_ms: u32,
        down_ms: u32,
        amplitude: f32,
        smooth: bool,
    ) -> f32 {
        let value = if t_ms < up_ms {
            // Ramp up
            if smooth {
                amplitude * smooth_ramp(t_ms as f32 / up_ms as f32)
            } else {
                amplitude * (t_ms as f32 / up_ms as f32)
            }
        } else if t_ms < up_ms + hold_ms {
            // Hold
            amplitude
        } else if t_ms < up_ms + hold_ms + down_ms {
            // Ramp down
            let t = (t_ms - up_ms - hold_ms) as f32;
            if smooth {
                amplitude * smooth_ramp(1.0 - t / down_ms as f32)
            } else {
                amplitude * (1.0 - t / down_ms as f32)
            }
        } else {
            // Done
            0.0
        };
        value
    }
    fn smooth_ramp(x: f32) -> f32 {
        0.5 - 0.5 * (core::f32::consts::PI * x).cos()
    }
    #[test]
    fn test_ringBuf() {
        let mut _v_t: Vec<u32> = vec![];
        let mut counts: Vec<u32> = vec![];
        let mut prev: Vec<f32> = vec![];

        let mut count: RingBuf<u32, 4> = RingBuf::new(0);
        for t in 0..4000 {
            let ramp_value = ramp_hold_ramp(t, 200, 400, 200, 2000.0, true); // ramp up 200ms, hold 400ms, ramp down 200ms

            count.push(0x8000 + (ramp_value as u32));

            _v_t.push(t);
            counts.push(match count.curr() {
                Some(v) => v,
                None => 0,
            });
            prev.push(match count.prev() {
                Some(v) => v,
                None => 0,
            } as f32);
        }

        let mut fg = Figure::new();
        fg.set_multiplot_layout(3, 1);
        fg.axes2d().lines(&_v_t, &counts, &[Caption("counts")]);
        fg.axes2d().lines(&_v_t, &prev, &[Caption("prev")]);

        // This will only run during `cargo test`
        let res = fg.show().unwrap();
        assert!(true);
    }
    #[test]
    fn test_constants() {
        eprint!("COUNT_PER_REV: {}\n\r", COUNT_PER_REVI32);
        eprint!("NUM:   {}\n\r", I16F16::from_num(0.00503).to_bits());

        eprint!("NUM:   {}\n\r", I32F32::from_num(0.00503).to_bits());

        // let num = I16F16::from_num(COUNT_PER_REVI32 / TWO_PI);
        // eprint!("RAD_TO_COUNTS bits: {}\n\r", num.to_bits());
        pub const OMEGA_ALPHA: I16F16 = I16F16::from_bits(10000); // 0.2 → moderate smoothing
        pub const ALPHA_ALPHA: I16F16 = I16F16::from_bits(20000); //3277 0.05 → heavier smoothing for alpha

        eprint!("OMEGA_ALPHA:   {}\n\r", OMEGA_ALPHA);
        eprint!("ALPHA_ALPHA:   {}\n\r", ALPHA_ALPHA);

        pub const OMEGA_EPS: I16F16 = I16F16::from_bits(100); // 0.05 rad/s
        pub const ALPHA_EPS: I16F16 = I16F16::from_bits(200); //48768 0.5 rad/s²
        eprint!("OMEGA_EPS:   {}\n\r", OMEGA_EPS);
        eprint!("ALPHA_EPS:   {}\n\r", ALPHA_EPS);
    }

    #[test]
    fn test_encoder() {
        let mut _v_t: Vec<u32> = vec![];

        let mut test_encoder = Encoder::new(MockEncoder { counter: 0xFFFF });
        let mut v_counts_raw: Vec<f32> = vec![];

        let mut v_counts: Vec<f32> = vec![];

        let mut vf32_theta: Vec<f32> = vec![];
        let mut vf32_omega: Vec<f32> = vec![];
        let mut vf32_alpha: Vec<f32> = vec![];
        let mut turns = 0;
        let mut t_last = 0;
        for t in 0..1000 {
            _v_t.push(t);

            let mut ramp_value = ramp_hold_ramp(t, 200, 400, 200, 5000.0, false) as i32; // ramp up 200ms, hold 400ms, ramp down 200ms

            ramp_value = ramp_value - 1250 * turns;
            // If the jump is more than half the revolution, it wrapped
            if ramp_value > COUNT_PER_REVI32 {
                turns += 1;
            } else if -ramp_value > COUNT_PER_REVI32 {
                turns -= 1;
            }

            let count = 0x8000 + ramp_value;
            test_encoder.write_enc_counter(count as u32);
            test_encoder.read_counter();
            v_counts_raw.push(test_encoder.prev_enc_counts as f32);
            test_encoder.update((t - t_last) * 8);
            t_last = t;
            v_counts.push(match test_encoder.counts.curr() {
                Some(v) => v,
                None => 0,
            } as f32);

            vf32_theta.push(test_encoder.theta.to_num::<f32>());
            vf32_omega.push(test_encoder.omega.to_num::<f32>());
            vf32_alpha.push(test_encoder.alpha.to_num::<f32>());
        }

        let mut fg = Figure::new();
        fg.set_multiplot_layout(5, 1);
        fg.axes2d()
            .lines(&_v_t, &v_counts_raw, &[Caption("counts raw")]);
        fg.axes2d().lines(&_v_t, &v_counts, &[Caption("counts")]);
        fg.axes2d().lines(&_v_t, &vf32_theta, &[Caption("theta")]);
        fg.axes2d().lines(&_v_t, &vf32_omega, &[Caption("omega")]);
        fg.axes2d().lines(&_v_t, &vf32_alpha, &[Caption("alpha")]);

        // This will only run during `cargo test`
        let res = fg.show().unwrap();
        eprint!("Omega:    {:?}\n\r", vf32_omega);
        eprint!("t:    {:?}\n\r", _v_t);

        assert!(true);
    }

    #[test]
    fn test_plot_output() {
        let x = [0u32, 1, 2];
        let y = [3u32, 4, 5];
        let mut fg = Figure::new();
        fg.axes2d()
            .lines(&x, &y, &[Caption("A line"), Color(gnuplot::Black)]);
        // This will only run during `cargo test`
        let res = fg.show().unwrap();
    }
}

#[cfg(test)]
mod encoder_tuning_tests {

    use fixed::types::I32F32;

    /// Test different filter parameters and visualize results
    #[test]
    fn tune_filter_parameters() {
        // Test configurations
        let test_configs = vec![
            // (OMEGA_ALPHA, ALPHA_ALPHA, OMEGA_EPS, ALPHA_EPS, description)
            (
                I32F32::from_num(0.1),
                I32F32::from_num(0.1),
                I32F32::from_num(0.01),
                I32F32::from_num(0.1),
                "Heavy filtering",
            ),
            (
                I32F32::from_num(0.3),
                I32F32::from_num(0.3),
                I32F32::from_num(0.01),
                I32F32::from_num(0.1),
                "Medium filtering",
            ),
            (
                I32F32::from_num(0.5),
                I32F32::from_num(0.5),
                I32F32::from_num(0.01),
                I32F32::from_num(0.1),
                "Light filtering",
            ),
            (
                I32F32::from_num(0.7),
                I32F32::from_num(0.7),
                I32F32::from_num(0.01),
                I32F32::from_num(0.1),
                "Minimal filtering",
            ),
            (
                I32F32::from_num(0.3),
                I32F32::from_num(0.3),
                I32F32::from_num(0.001),
                I32F32::from_num(0.01),
                "Tight deadband",
            ),
            (
                I32F32::from_num(0.3),
                I32F32::from_num(0.3),
                I32F32::from_num(0.1),
                I32F32::from_num(1.0),
                "Wide deadband",
            ),
        ];

        for (omega_alpha, alpha_alpha, omega_eps, alpha_eps, desc) in test_configs {
            println!("\n========== {} ==========", desc);
            println!("OMEGA_ALPHA: {}, ALPHA_ALPHA: {}", omega_alpha, alpha_alpha);
            println!("OMEGA_EPS: {}, ALPHA_EPS: {}", omega_eps, alpha_eps);

            let results = run_simulation(omega_alpha, alpha_alpha, omega_eps, alpha_eps);
            analyze_results(&results, desc);
        }
    }

    /// Run encoder simulation with given parameters
    /// Run encoder simulation with given parameters
    fn run_simulation(
        omega_alpha: I32F32,
        alpha_alpha: I32F32,
        omega_eps: I32F32,
        alpha_eps: I32F32,
    ) -> Vec<(I32F32, I32F32, I32F32)> {
        let mut results = Vec::new();

        let mut omega = I32F32::from_bits(0);
        let mut alpha = I32F32::from_bits(0);
        let mut prev_omega = I32F32::from_bits(0);

        // Simulate encoder data with constant velocity
        let counts: Vec<(u32, u32)> = (0..1000)
            .map(|i| {
                let count = (i * 100) as u32; // Constant velocity
                let timestamp = (i * 24000) as u32; // 24000 ticks = 1ms at 24MHz
                (count, timestamp)
            })
            .collect();

        for i in 1..counts.len() {
            let (c0, t0) = counts[i];
            let (c1, t1) = counts[i - 1];

            let dc1 = c0.wrapping_sub(c1) as i32;
            let dt_ticks = t0.wrapping_sub(t1) as i32;

            if dt_ticks <= 0 {
                continue;
            }

            // Calculate omega: dc1 / dt where dt is in seconds
            // omega = dc1 * (ticks_per_second / dt_ticks)
            //       = dc1 * (24_000_000 / dt_ticks)
            // To avoid overflow, calculate: (dc1 * 24_000_000) / dt_ticks
            // But use float for the division to avoid overflow

            let omega_raw_f64 = (dc1 as f64) / (dt_ticks as f64) * 24_000_000.0;
            let mut omega_raw = I32F32::from_num(omega_raw_f64);

            // Apply deadband
            if omega_raw.abs() < omega_eps {
                omega_raw = I32F32::from_bits(0);
            }

            // Filtered omega
            omega = omega_alpha * omega + (I32F32::from_num(1) - omega_alpha) * omega_raw;

            // Calculate alpha (acceleration)
            let alpha_raw_f64 =
                ((omega - prev_omega).to_num::<f64>()) / (dt_ticks as f64) * 24_000_000.0;
            let mut alpha_raw = I32F32::from_num(alpha_raw_f64);

            // Apply deadband
            if alpha_raw.abs() < alpha_eps {
                alpha_raw = I32F32::from_bits(0);
            }

            // Filtered alpha
            alpha = alpha_alpha * alpha + (I32F32::from_num(1) - alpha_alpha) * alpha_raw;

            prev_omega = omega;

            results.push((omega, alpha, omega_raw));
        }

        results
    }

    /// Analyze and print statistics
    fn analyze_results(results: &[(I32F32, I32F32, I32F32)], desc: &str) {
        if results.is_empty() {
            println!("No results!");
            return;
        }

        // Calculate statistics
        let mut omega_sum = I32F32::from_bits(0);
        let mut alpha_sum = I32F32::from_bits(0);
        let mut omega_variance = I32F32::from_bits(0);
        let mut alpha_variance = I32F32::from_bits(0);

        // Skip initial settling period (first 10%)
        let settle_idx = results.len() / 10;
        let steady_state = &results[settle_idx..];

        for (omega, alpha, _) in steady_state {
            omega_sum += *omega;
            alpha_sum += *alpha;
        }

        let omega_mean = omega_sum / I32F32::from_num(steady_state.len());
        let alpha_mean = alpha_sum / I32F32::from_num(steady_state.len());

        for (omega, alpha, _) in steady_state {
            let omega_diff = *omega - omega_mean;
            let alpha_diff = *alpha - alpha_mean;
            omega_variance += omega_diff * omega_diff;
            alpha_variance += alpha_diff * alpha_diff;
        }

        omega_variance /= I32F32::from_num(steady_state.len());
        alpha_variance /= I32F32::from_num(steady_state.len());

        println!("Settling time: {} samples", settle_idx);
        println!("Steady-state omega mean: {}", omega_mean);
        println!("Steady-state omega variance: {}", omega_variance);
        println!("Steady-state alpha mean: {}", alpha_mean);
        println!("Steady-state alpha variance: {}", alpha_variance);

        // Find jumps (spikes larger than 2x mean)
        let mut jump_count = 0;
        for i in 1..results.len() {
            let omega_change = (results[i].0 - results[i - 1].0).abs();
            if omega_change > omega_mean.abs() * I32F32::from_num(2) {
                jump_count += 1;
            }
        }
        println!("Number of omega jumps: {}", jump_count);
    }

    /// Test with actual noisy encoder data
    #[test]
    fn test_with_noisy_data() {
        use rand::Rng;
        let mut rng = rand::thread_rng();

        let omega_alpha = I32F32::from_num(0.3);
        let alpha_alpha = I32F32::from_num(0.3);
        let omega_eps = I32F32::from_num(0.01);
        let alpha_eps = I32F32::from_num(0.1);

        let mut omega = I32F32::from_bits(0);
        let mut alpha = I32F32::from_bits(0);
        let mut prev_omega = I32F32::from_bits(0);

        let mut omega_history = Vec::new();
        let mut alpha_history = Vec::new();

        // Generate noisy encoder data
        for i in 1..1000 {
            let noise = rng.gen_range(-5..=5);
            let count_current = (i * 100 + noise) as u32;
            let count_prev = ((i - 1) * 100) as u32;

            let dc1 = count_current.wrapping_sub(count_prev) as i32;
            let dt_ticks = 24000; // 1ms at 24MHz

            // Calculate omega using f64 intermediate
            let omega_raw_f64 = (dc1 as f64) / (dt_ticks as f64) * 24_000_000.0;
            let mut omega_raw = I32F32::from_num(omega_raw_f64);

            if omega_raw.abs() < omega_eps {
                omega_raw = I32F32::from_bits(0);
            }
            omega = omega_alpha * omega + (I32F32::from_num(1) - omega_alpha) * omega_raw;

            // Calculate alpha
            let alpha_raw_f64 =
                ((omega - prev_omega).to_num::<f64>()) / (dt_ticks as f64) * 24_000_000.0;
            let mut alpha_raw = I32F32::from_num(alpha_raw_f64);

            if alpha_raw.abs() < alpha_eps {
                alpha_raw = I32F32::from_bits(0);
            }
            alpha = alpha_alpha * alpha + (I32F32::from_num(1) - alpha_alpha) * alpha_raw;

            prev_omega = omega;

            omega_history.push(omega);
            alpha_history.push(alpha);
        }

        println!("\nNoisy data test:");
        let len = omega_history.len();
        if len >= 10 {
            println!("Omega (last 10): {:?}", &omega_history[len - 10..len]);
            println!("Alpha (last 10): {:?}", &alpha_history[len - 10..len]);
        } else {
            println!("Omega: {:?}", omega_history);
            println!("Alpha: {:?}", alpha_history);
        }
    }
    /// Generate tuning recommendation based on your actual data
    #[test]
    fn analyze_your_data() {
        // Your actual data from the output (convert to I32F32)
        let omega_data: Vec<I32F32> = vec![
            I32F32::from_num(0.0),
            I32F32::from_num(-11.215393),
            I32F32::from_num(-12.926727),
            I32F32::from_num(-13.187866),
            I32F32::from_num(-13.227707),
            I32F32::from_num(-13.233795),
            I32F32::from_num(-13.234711),
            I32F32::from_num(-13.234863),
            I32F32::from_num(-13.234879),
            I32F32::from_num(-13.234879),
            I32F32::from_num(-13.234879),
        ];

        println!("\n========== Analysis of Your Data ==========");

        // Detect settling time
        let target = I32F32::from_num(-13.234879);
        let tolerance = I32F32::from_num(0.01);
        let mut settling_idx = 0;
        for (i, &val) in omega_data.iter().enumerate() {
            if (val - target).abs() < tolerance {
                settling_idx = i;
                break;
            }
        }
        println!("Settling time: {} samples", settling_idx);

        // Detect jumps
        let jump_threshold = I32F32::from_num(1.0);
        let jumps: Vec<_> = omega_data
            .windows(2)
            .enumerate()
            .filter(|(_, w)| (w[1] - w[0]).abs() > jump_threshold)
            .collect();

        println!("Number of large jumps (>1.0): {}", jumps.len());

        // Calculate max jump
        let max_jump = omega_data
            .windows(2)
            .map(|w| (w[1] - w[0]).abs())
            .max()
            .unwrap_or(I32F32::from_bits(0));
        println!("Maximum jump: {}", max_jump);

        // Recommendations
        println!("\nRecommendations:");
        if settling_idx > 10 {
            println!("- Increase OMEGA_ALPHA for faster settling (try 0.4-0.6)");
        }
        if jumps.len() > 5 {
            println!("- Increase OMEGA_EPS to filter noise (try 0.05-0.1)");
            println!("- Decrease OMEGA_ALPHA for more smoothing (try 0.2-0.3)");
        }

        println!("\nSuggested starting values for I32F32:");
        println!(
            "const OMEGA_ALPHA: I32F32 = I32F32::from_bits({});",
            (0.3 * 4294967296.0) as i64
        );
        println!(
            "const OMEGA_EPS: I32F32 = I32F32::from_bits({});",
            (0.05 * 4294967296.0) as i64
        );
        println!(
            "const ALPHA_ALPHA: I32F32 = I32F32::from_bits({});",
            (0.2 * 4294967296.0) as i64
        );
        println!(
            "const ALPHA_EPS: I32F32 = I32F32::from_bits({});",
            (0.5 * 4294967296.0) as i64
        );
    }
}
