use crate::*;
use bitfield_struct::bitfield;
use core::marker::PhantomData;
use ffi::*;
use fixed::{consts, types::I16F16, FixedI32};

#[derive(Copy, Clone)]
pub struct RingBuf {
    buf: [I16F16; 3],
    idx: usize,
}

impl RingBuf {
    const ZERO: I16F16 = I16F16::from_bits(0);

    const fn new() -> Self {
        Self {
            buf: [Self::ZERO; 3],
            idx: 0,
        }
    }

    #[inline(always)]
    pub fn push(&mut self, v: I16F16) {
        self.buf[self.idx] = v;
        self.idx = (self.idx + 1) % 3;
    }

    /// Most recent sample
    #[inline(always)]
    pub fn curr(&self) -> I16F16 {
        self.buf[(self.idx + 2) % 3]
    }

    /// Previous sample
    #[inline(always)]
    pub fn prev(&self) -> I16F16 {
        self.buf[(self.idx + 1) % 3]
    }

    /// Two samples ago
    #[inline(always)]
    pub fn prev2(&self) -> I16F16 {
        self.buf[self.idx]
    }
}

pub trait EncoderOps {
    fn init_hardware(&self);
    fn start_hardware(&self);
    fn write_counter(&self, value: u32);
    fn read_counter(&self) -> u32;
}

pub struct XEncoder;
pub struct YEncoder;

impl EncoderOps for XEncoder {
    fn init_hardware(&self) {
        unsafe { DecX_Init() }
    }
    fn start_hardware(&self) {
        unsafe { DecX_Start() }
    }
    fn write_counter(&self, value: u32) {
        unsafe { DecX_WriteCounter(value) }
    }

    fn read_counter(&self) -> u32 {
        unsafe { DecX_ReadCounter() } // call your C binding
    }
}

// impl EncoderOps for YEncoder {
//     fn init_hardware(&self) {
//         unsafe { DecY_Init() }
//     }
//     fn start_hardware(&self) {
//         unsafe { DecY_Start() }
//     }
//     fn write_counter(&self, value: u32) {
//         unsafe { DecY_WriteCounter(value) }
//     }
//     fn read_counter(&self) -> u32 {
//         unsafe { DecY_ReadCounter() } // call your C binding
//     }
// }
pub struct Encoder<T: EncoderOps> {
    pub curr: u32,  // current raw counter value from hardware
    pub last: u32,  // last raw counter value
    pub turns: i32, // number of overflows/underflows counted
    pub dir: u8,    // direction: 1=backward, 2=forward
    pub theta: RingBuf,
    pub omega: RingBuf,
    pub alpha: RingBuf,
    ops: T,
}
const STEPS_PER_REV: i32 = 3200;
// Fixed-point constants
const TS: i32 = 5;

impl<T: EncoderOps> Encoder<T> {
    pub fn new(ops: T) -> Self {
        ops.init_hardware();
        ops.start_hardware();
        Self {
            curr: 0,
            last: 0,
            turns: 0,
            dir: 0,
            theta: RingBuf::new(),
            omega: RingBuf::new(),
            alpha: RingBuf::new(),
            ops,
        }
    }
    /// Returns the init of this [`Encoder`].
    pub fn init(&mut self) {
        self.curr = 0;
        self.last = 0;
        self.turns = 0;
        self.dir = 0;
        self.theta = RingBuf::new();
        self.omega = RingBuf::new();
        self.alpha = RingBuf::new();
        self.ops.init_hardware();
        self.ops.start_hardware();
        self.ops.write_counter(0);
        self.ops.read_counter();
    }
    /// Read the current encoder counter and update position
    pub fn update(&mut self) -> bool {
        self.curr = self.ops.read_counter();
        if self.curr == self.last {
            return false; // no change
        }

        // small movement, normal direction
        if (self.curr as i32 - self.last as i32).abs() < 1000 {
            self.dir = if self.curr > self.last { 2 } else { 1 };
        } else {
            // handle overflow / underflow
            if self.curr > self.last {
                // counter wrapped backward (underflow)
                self.turns -= 1;
            } else {
                // counter wrapped forward (overflow)
                self.turns += 1;
            }
        }

        self.last = self.curr;

        // Calculate position (θ)
        self.theta
            .push(I16F16::from_num(self.curr as i32 * STEPS_PER_REV as i32));

        // Calculate velocity (ω) using finite difference
        let raw_omega = (self.theta.curr() - self.theta.prev()) / TS;
        let raw_alpha = (self.omega.curr() - self.omega.prev()) / TS;

        // Apply simple low-pass filter to ω
        self.omega
            .push(self.omega.prev() + ((raw_omega - self.omega.prev()) >> 3)); // α = 1/8

        // Apply simple low-pass filter to α

        // Calculate acceleration (α) using finite difference
        self.alpha
            .push(self.alpha.prev() + ((raw_alpha - self.alpha.prev()) >> 3)); // α = 1/8

        uart_println!(
            "{},{},{}\n\r",
            self.theta.curr(),
            self.omega.curr(),
            self.alpha.curr()
        );

        true
    }

    /// Full encoder position as i32
    pub fn _read_position(&mut self) -> i32 {
        self.read_counter();

        return self.turns * 32768 + self.curr as i32; // assuming a 16-bit counter
    }

    pub fn get_speed(&self) -> I16F16 {
        self.omega.curr()
    }
    pub fn get_acc(&self) -> I16F16 {
        self.alpha.curr()
    }

    /// Reads the current absolute position from the hardware counter.
    pub fn read_counter(&mut self) {
        self.curr = self.ops.read_counter() // call your C binding
    }
    /// Reads the change in position since the last call to this function
    /// (or since `new()` if called once).
    pub fn _read_delta(&mut self) -> u32 {
        self.read_counter();

        let delta = self.curr - self.last;
        self.last = self.curr;
        delta
    }

    /// Resets the encoder counter to a specific value.
    pub fn _set_position(&mut self, value: u32) {
        self.last = value;
        self.ops.write_counter(value);
    }
}

#[no_mangle]
extern "C" fn Pulser_InterruptHandler() {
    unsafe {
        /* Clears the Timer terminal count interrupt */
        Xaxis.get_mut().run(&mut SYS.get_mut().step_out);
        StepReg_Write(SYS.get().step_out);
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
#[derive(PartialEq, Clone)]

pub enum MotorState {
    IDLE,
    _DISABLE,
    ACCEL,
    CONST_SPD,
    DECEL,
    _ERROR,
}
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
    step_pin: u8,
    target_pos_steps: i32,  // Target speed in Hz
    curr_pos_steps: i32,    // Target speed in Hz
    target_speed_hz: u32,   // Target speed in Hz
    current_speed_hz: u32,  // Current speed in Hz
    acceleration_hz_s: u32, // Acceleration in Hz/s
    deceleration_hz_s: u32, // Deceleration in Hz/s
    step_interval: u32,     // Current step interval (ms)
    timer: u32,             // Last step time (ms)
}

impl<T: EncoderOps> Stepper<T> {
    pub fn new(ops: T, ix: u8) -> Self {
        Self {
            encoder: Encoder::new(ops),
            state: MotorState::IDLE,
            // target_pos: None,
            dir: MotorDirection::FWD,
            target_pos_steps: 0,
            curr_pos_steps: 0,
            step_pin: ix,
            target_speed_hz: 1000,
            current_speed_hz: 0,
            acceleration_hz_s: 5000,
            deceleration_hz_s: 5000,
            step_interval: 1000, // Start with 1Hz (1000ms interval,
            timer: 0,
        }
    }
    pub fn init_encoder(&mut self) {
        self.encoder.init();
    }
    pub fn set_target_position(&mut self, position_steps: i32) {
        self.target_pos_steps = position_steps;
    }
    pub fn get_current_position(&self) -> i32 {
        self.curr_pos_steps
    }
    pub fn set_speed(&mut self, speed_hz: u32) {
        // self.target_speed_hz = speed_hz;
        self.target_speed_hz = speed_hz * 10 / 33;
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
    pub fn UpdateSpd(&mut self) {
        if self.current_speed_hz < self.target_speed_hz && self.current_speed_hz > 0 {}
        match self.state {
            MotorState::ACCEL => {
                // Accelerate: increase speed until target is reached
                uart_printf(format_args!("\n\rACC:{}", self.current_speed_hz));

                self.current_speed_hz = self
                    .current_speed_hz
                    .saturating_add(self.acceleration_hz_s / 1000); // Hz/s to Hz/ms
                if self.current_speed_hz >= self.target_speed_hz {
                    self.current_speed_hz = self.target_speed_hz;
                    self.state = MotorState::CONST_SPD;
                }
            }
            MotorState::DECEL => {
                uart_printf(format_args!("\n\rDEC:{}", self.current_speed_hz));

                //  Decelerate: decrease speed until 0
                self.current_speed_hz = self
                    .current_speed_hz
                    .saturating_sub(self.deceleration_hz_s / 1000); // Hz/s to Hz/ms
                if self.current_speed_hz <= 0 {
                    self.current_speed_hz = 0;
                    self.state = MotorState::IDLE;
                }
            }
            MotorState::CONST_SPD => {
                uart_printf(format_args!("\n\rCTE:{}", self.current_speed_hz));
            }
            _ => {}
        }

        // Update step interval (ms)
        self.step_interval = if self.current_speed_hz > 0 {
            1000 / self.current_speed_hz
        } else {
            1000
        };
    }
    /// Sets the speed/frequency of the step pulses.
    /// `period` corresponds to the timer/PWM period register value.
    pub fn set_speed_period(&mut self, period: u32) {
        self.target_speed_hz = period;
    }

    /// Stops the motor movement.
    pub fn stop(&mut self) {
        self.state = MotorState::IDLE;
        self.target_speed_hz = 0;
    }
    pub fn run(&mut self, out: &mut u8) -> bool {
        self.encoder.update();
        match self.state {
            MotorState::ACCEL | MotorState::DECEL | MotorState::CONST_SPD => {
                self.UpdateSpd();
                self.timer += 50;
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
        false
    }
}

#[cfg(test)]
#[cfg(not(target_arch = "arm"))]
mod Motor_tests {
    use std::{iter, ops::Range};

    use super::*;
    use motor::*;
    #[test]
    fn test_stepper() {
        let mut motor = Stepper::new(0);
        motor.set_direction(MotorDirection::FWD);
        motor.set_speed_period(100);
        for i in 0..10 {
            let mut out: u8 = 0;
            motor.run(&mut out);
            assert_eq!(out, 0);
            motor.run(&mut out);
            assert_eq!(out & 1, 1); // step pulse generated
        }
    }

    use gnuplot::{Caption, Color, Figure};
    fn rangetest<T>(iter: &Vec<T>, f: &dyn Fn(T) -> T) {
        let mut x: Vec<T> = vec![];
        let mut y: Vec<T> = vec![];

        for i in iter {
            x.push(*i.clone());
            y.push(f(*i));
        }
    }
    #[test]
    fn test_plot_output() {
        let mut fg = Figure::new();

        let x = [0u32, 1, 2];
        let y = [3u32, 4, 5];
        let mut fg = Figure::new();
        fg.axes2d()
            .lines(&x, &y, &[Caption("A line"), Color(gnuplot::Black)]);
        // This will only run during `cargo test`
        let res = fg.show().unwrap();
    }
}
