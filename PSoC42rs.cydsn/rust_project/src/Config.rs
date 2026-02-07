use fixed::{consts, types::I16F16, types::I32F32, FixedI32};

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
