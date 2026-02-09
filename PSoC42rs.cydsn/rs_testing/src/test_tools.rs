use rust_core::encoder_core::*;

//     counts
//   ^
//   |        ┌───────┐
//   |       /         \
//   |______/           \______
//            time →
pub fn ramp_hold_ramp(
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
pub fn smooth_ramp(x: f32) -> f32 {
    0.5 - 0.5 * (core::f32::consts::PI * x).cos()
}

pub struct MockEncoder {
    pub counter: u32,
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
