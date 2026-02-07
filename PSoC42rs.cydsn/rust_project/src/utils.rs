use core::num::Wrapping;
use core::ops::Sub;
use fixed::{consts, types::I16F16, types::I32F32, FixedI32};

#[derive(Copy, Clone)]
pub struct IirFilter {
    y: I16F16,
    alpha: I16F16,
}

impl IirFilter {
    pub const fn new(alpha: I16F16) -> Self {
        Self {
            y: I16F16::from_bits(0),
            alpha,
        }
    }

    #[inline(always)]
    pub fn update(&mut self, x: I16F16) -> I16F16 {
        self.y += self.alpha * (x - self.y);
        self.y
    }
}

#[derive(Copy, Clone)]
pub struct RingBuf<T, const N: usize> {
    buf: [T; N],
    idx: usize,
    len: usize,
}

impl<T: Copy, const N: usize> RingBuf<T, N> {
    const check: () = assert!(N.is_power_of_two());
    /// Creates a new `RingBuf` with all elements set to `zero`.
    /// Requires `T` to implement `Copy` and `From<i32>` (or similar) for `zero`.
    pub const fn new(zero: T) -> Self {
        Self {
            buf: [zero; N],
            idx: 0,
            len: 0,
        }
    }

    /// Pushes a new value into the ring buffer.
    #[inline(always)]
    pub fn push(&mut self, v: T) {
        self.buf[self.idx] = v;
        self.idx = (self.idx + 1) & (N - 1); //& (N - 1) instead of %N faster for M0
        if self.len < N {
            self.len += 1;
        }
    }

    /// Returns the most recent sample.
    #[inline(always)]
    pub fn curr(&self) -> Option<T> {
        (self.len >= 1).then(|| self.buf[self.idx.wrapping_sub(1) & (N - 1)])
    }

    #[inline(always)]
    pub fn prev(&self) -> Option<T> {
        (self.len >= 2).then(|| self.buf[self.idx.wrapping_sub(2) & (N - 1)])
    }

    #[inline(always)]
    pub fn prev2(&self) -> Option<T> {
        (self.len >= 3).then(|| self.buf[self.idx.wrapping_sub(3) & (N - 1)])
    }
    #[inline(always)]
    pub fn get_last_two(&self) -> Option<(T, T)> {
        if self.len >= 2 {
            let first = self.buf[self.idx.wrapping_sub(1) & (N - 1)];
            let second = self.buf[self.idx.wrapping_sub(2) & (N - 1)];
            Some((first, second))
        } else {
            None
        }
    }
}
