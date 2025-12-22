use crate::Xaxis;
use crate::*;
use bitfield_struct::bitfield;

use ffi::*;
#[derive(PartialEq, Clone)]
#[repr(u8)]
#[cfg_attr(not(target_arch = "arm"), derive(Debug))]
pub enum System_State {
    IDLE,
    START_MOVE,
    MOVE_RAMP,
    MOVING,
    KILL,
    WAIT,
}

pub struct System_T {
    new_state: bool,
    last_state: System_State,
    pub state: System_State,
    pub next_state: System_State,
    tmr: u32,
    pub step_out: u8,
}
impl System_T {
    pub fn new() -> Self {
        System_T {
            new_state: false,
            last_state: System_State::WAIT,
            state: System_State::WAIT,
            next_state: System_State::IDLE,
            tmr: 0,
            step_out: 0,
        }
    }
    pub fn sys_task(&mut self) {
        self.state = self.next_state.clone();
        if self.state != self.last_state {
            self.new_state = true;
        }
        self.last_state = self.state.clone();
        match self.state {
            System_State::IDLE => {
                if self.new_state {
                    uart_put_str("\n\r-Idle\n\r");
                    self.new_state = false;
                }
            }
            System_State::MOVING => {
                if self.new_state {
                    self.new_state = false;
                    uart_put_str("\n\r-Moving\n\r");
                    self.tmr = 0;
                }
                // self.step_out = !self.step_out;
                // uart_printf(format_args!("\rSTEP_X {}", self.step_out));

                match Xaxis.get().state {
                    MotorState::ACCEL => {
                        // if self.tmr >= 2000 {
                        //     Xaxis.get_mut().state = MotorState::CONST_SPD;
                        //     self.tmr = 0;
                        //     uart_put_str("\n\r-END Acc\n\r");
                        // }
                    }
                    MotorState::CONST_SPD => {
                        self.tmr += 1;
                        if self.tmr >= 50000 {
                            Xaxis.get_mut().state = MotorState::DECEL;
                            self.tmr = 0;
                            uart_put_str("\n\r-End Const Speed\n\r");
                        }
                    }
                    MotorState::DECEL => {
                        // if self.tmr >= 2000 {
                        //     Xaxis.get_mut().state = MotorState::IDLE;
                        //     self.tmr = 0;
                        //     uart_put_str("\n\r-End DECEL\n\r");
                        // }
                    }
                    MotorState::IDLE => {
                        self.next_state = System_State::KILL;
                        // if self.tmr >= 2000 {
                        //     Xaxis.get_mut().state = MotorState::IDLE;
                        //     self.tmr = 0;
                        //     uart_put_str("\n\r-End DECEL\n\r");
                        // }
                    }
                    _ => self.next_state = System_State::IDLE,
                }
            }
            System_State::KILL => {
                if self.new_state {
                    self.new_state = false;
                    uart_put_str("\n\r-KILL\n\r");
                    Xaxis.get_mut().stop();
                    unsafe { EN_Write(1) };
                }
                self.next_state = System_State::IDLE;
            }
            System_State::START_MOVE => {
                if self.new_state {
                    self.new_state = false;
                    unsafe {
                        EN_Write(0);
                    }
                    uart_put_str("\n\r-Run\n\r");
                }

                Xaxis.get_mut().state = MotorState::ACCEL;

                self.next_state = System_State::MOVING;
            }
            System_State::MOVE_RAMP => {
                if self.new_state {
                    self.new_state = false;
                }
            }
            System_State::WAIT => {
                if self.new_state {
                    self.new_state = false;
                }
            }
        }
    }
}

#[cfg(test)]
#[cfg(not(target_arch = "arm"))]
mod sys_tests {
    use super::*;
    use sys::*;
    #[test]
    fn test_sys_task() {
        let mut sys = System_T::new();
        assert_eq!(sys.state, System_State::IDLE);
        sys.next_state = System_State::START_MOVE;
        sys.sys_task();
        assert_eq!(sys.state, System_State::START_MOVE);
        sys.next_state = System_State::MOVING;
        sys.sys_task();
        assert_eq!(sys.state, System_State::MOVING);
    }
}
