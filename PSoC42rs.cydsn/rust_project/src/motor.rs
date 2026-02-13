use crate::*;
// use core::marker::PhantomData;
use crate::SYS;
use ffi::*;
use fixed::{consts, types::I16F16, types::I32F32, FixedI32};
use rust_core::encoder_core::{Encoder, EncoderOps};

#[unsafe(no_mangle)]
extern "C" fn Pulser_InterruptHandler() {
    //10us
    unsafe {
        Xaxis.get_mut().run(&mut SYS.get_mut().step_out);
        StepReg_Write(SYS.get().step_out);
        /* Clears the Timer terminal count interrupt */
        Pulser_tmr_ClearInterrupt(Pulser_tmr__INTR_MASK_TC);
    }
}

pub fn pulser_init() {
    unsafe {
        ISR_Pulser_StartEx(Some(Pulser_InterruptHandler));
        Pulser_tmr_Start();
    }
    uart_printf(format_args!("pulser initialized.\n\r"));
}
#[derive(PartialEq, Clone, Debug)]

pub enum MotorState {
    IDLE,
    _DISABLE,
    ACCEL,
    CONST_SPD,
    DECEL,
    _ERROR,
}
#[derive(PartialEq, Clone)]
pub enum MotorDirection {
    FWD,
    BWD,
}
// In your application library or module
// #[bitfield(u8)]
pub struct Stepper<T: EncoderOps> {
    pub encoder: Encoder<T>,
    //    #[bits(4)]
    pub state: MotorState, // current raw counter value from hardware
    // pub target_pos: Option<u32>, // last raw counter value
    pub dir: MotorDirection,
    pub old_dir: MotorDirection,
    step_pin: u8,
    target_pos_steps: i32, // Target speed in Hz
    curr_pos_steps: i32,   // Target speed in Hz
    pub current_speed_hz: i64,
    pub curr_target_speed_hz: i64,
    pub target_speed_hz: i64,
    pub acceleration_hz_ms: i64, // Store as Hz/ms to avoid dividing by 1000 in the loop
    pub deceleration_hz_ms: i64,
    step_interval: u32, // Current step interval (ms)
    timer: u32,         // Last step time (ms)
}

impl<T: EncoderOps> Stepper<T> {
    pub fn new(ops: T, ix: u8) -> Self {
        Self {
            encoder: Encoder::new(ops),
            state: MotorState::IDLE,
            // target_pos: None,
            dir: MotorDirection::FWD,
            old_dir: MotorDirection::FWD,
            target_pos_steps: 0,
            curr_pos_steps: 0,
            step_pin: ix,
            target_speed_hz: 1000,

            curr_target_speed_hz: 1000,
            current_speed_hz: 0,
            acceleration_hz_ms: 1,
            deceleration_hz_ms: 1,
            step_interval: 1000, // Start with 1Hz (1000ms interval,
            timer: 0,
        }
    }

    pub fn set_target_position(&mut self, position_steps: i32) {
        self.target_pos_steps = position_steps;
    }
    pub fn get_current_position(&self) -> i32 {
        self.curr_pos_steps
    }
    pub fn set_speed(&mut self, speed_hz: u32) {
        if self.state != MotorState::IDLE {
            self.curr_target_speed_hz = (speed_hz as i64) / 2;
        }
        self.target_speed_hz = (speed_hz as i64) / 2;
    }
    /// Sets the motor movement direction.
    pub fn set_direction(&mut self, direction: MotorDirection) {
        unsafe {
            match direction {
                MotorDirection::FWD => DIR_Write(0),
                MotorDirection::BWD => DIR_Write(1),
            };
        }
        self.dir = direction;
    }

    // Update motor speed based on acceleration/deceleration
    // 32us
    pub fn update_spd(&mut self) {
        if self.old_dir != self.dir {
            if self.current_speed_hz != 0 {
                //need to decel to 0 first
                self.curr_target_speed_hz = 0;
            } else {
                self.state = MotorState::CONST_SPD;
                let dir = match self.dir {
                    MotorDirection::FWD => 1,
                    MotorDirection::BWD => -1,
                };
                self.curr_target_speed_hz = dir * self.target_speed_hz;
                Xaxis.get_mut().set_direction(self.dir.clone());
                self.old_dir = self.dir.clone();
            }
        }
        match self.state {
            MotorState::ACCEL => {
                self.current_speed_hz = self
                    .current_speed_hz
                    .saturating_add(self.acceleration_hz_ms)
                    .min(self.curr_target_speed_hz);

                if self.current_speed_hz == self.curr_target_speed_hz {
                    self.state = MotorState::CONST_SPD;
                }
            }
            MotorState::DECEL => {
                self.current_speed_hz = self
                    .current_speed_hz
                    .saturating_sub(self.deceleration_hz_ms)
                    .max(self.curr_target_speed_hz);

                if self.current_speed_hz == self.curr_target_speed_hz {
                    self.state = MotorState::CONST_SPD;
                }
            }
            MotorState::CONST_SPD => {
                self.state = match self.curr_target_speed_hz.cmp(&self.current_speed_hz) {
                    core::cmp::Ordering::Greater => MotorState::ACCEL,
                    core::cmp::Ordering::Less => MotorState::DECEL,
                    core::cmp::Ordering::Equal if self.curr_target_speed_hz == I16F16::ZERO => {
                        MotorState::IDLE
                    }
                    _ => MotorState::CONST_SPD,
                };
            }

            _ => {}
        }

        //Pulser Period = 1/24  1200 period -> 50us min, 20ms max
        // Update step interval (ms)
        let speed_int: u32 = self.current_speed_hz.abs() as u32;
        self.step_interval = if speed_int == 0 {
            50 //fastest
        } else {
            (30000_u32 / speed_int).clamp(1, 3000)
        };
    }
    pub fn control_stop(&mut self) {
        self.state = MotorState::DECEL;
        self.curr_target_speed_hz = 0;
    }
    /// Stops the motor movement.
    pub fn stop(&mut self) {
        self.state = MotorState::IDLE;
        self.curr_target_speed_hz = 0;
        self.current_speed_hz = 0;
    }
    pub fn run(&mut self, out: &mut u8) {
        // idle 2.88us
        //moving 43.81us

        match self.state {
            MotorState::ACCEL | MotorState::DECEL | MotorState::CONST_SPD => {
                self.update_spd();
                self.timer += 1;
                let mask = 1 << self.step_pin;
                if self.timer >= self.step_interval {
                    *out |= mask;
                    self.timer = 0;
                } else {
                    *out &= !mask;
                }
            }
            _ => {}
        }
    }
}

#[cfg(not(target_arch = "arm"))]
#[cfg(test)]
mod tests {
    use super::*;
    use crate::motor::*;
    use crate::Config::*;
    // Helper to create a default motor instance
    fn setup() -> Stepper<XEncoder> {
        let mut test_encoder = Encoder::new(XEncoder);
        let mut motor = Stepper::new(XEncoder, 0);
        motor.acceleration_hz_ms = 15;
        motor.deceleration_hz_ms = 15;
        motor.step_interval = 20000;
        motor
    }

    #[test]
    fn test_accel_clamping_at_target() {
        let mut motor = setup();

        motor.target_speed_hz = 5;
        motor.current_speed_hz = 4;
        motor.state = MotorState::ACCEL;
        // Cycle 1: 0.0 + 1.5 = 1.5
        motor.update_spd();
        assert_eq!(motor.current_speed_hz, 5);
        assert_eq!(motor.state, MotorState::CONST_SPD);
    }
    #[test]
    fn test_decel_to_idle() {
        let mut motor = setup();

        motor.state = MotorState::DECEL;
        motor.current_speed_hz = 3;
        motor.target_speed_hz = 0;

        // Update 1: 3.0 - 2.0 = 1.0
        motor.update_spd();
        assert_eq!(motor.current_speed_hz, 15);
        assert_eq!(motor.state, MotorState::DECEL);

        // Update 2: 1.0 - 2.0 = 0 (saturating), state should be IDLE
        motor.update_spd();
        assert_eq!(motor.current_speed_hz, 0);
        assert_eq!(motor.state, MotorState::IDLE);
    }

    #[test]
    fn test_mid_air_reversal() {
        let mut motor = setup();

        motor.state = MotorState::CONST_SPD;
        motor.current_speed_hz = 50;

        // Target is suddenly much lower
        motor.target_speed_hz = 485;
        motor.update_spd();

        // Should switch to DECEL immediately
        assert_eq!(motor.state, MotorState::DECEL);

        // Should have reduced speed in the same cycle if update_spd
        // logic allows it, or next cycle. Based on the logic provided:
        // The first call switches state, the SECOND call starts the ramp.
        motor.update_spd();
        assert_eq!(motor.current_speed_hz, 485);
    }

    #[test]
    fn test_step_interval_math() {
        let mut motor = setup();

        // Speed = 100Hz -> Interval = 100,000 / 100 = 1000
        motor.current_speed_hz = 100;
        motor.update_spd();
        assert_eq!(motor.step_interval, 1000);

        // Speed = 0Hz -> Interval should be the default 20,000
        motor.current_speed_hz = 0;
        motor.update_spd();
        assert_eq!(motor.step_interval, 20000);
    }
}
