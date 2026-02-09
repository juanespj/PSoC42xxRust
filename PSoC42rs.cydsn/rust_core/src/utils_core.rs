#[cfg(target_arch = "arm")]
use core::option::{Option, Option::*};
use core::prelude::rust_2024::*; // for #[derive] support

#[derive(Copy, Clone)]
pub struct RingBuf<T, const N: usize> {
    buf: [T; N],
    idx: usize,
    len: usize,
}

impl<T: Copy, const N: usize> RingBuf<T, N> {
    const fn check() {
        if !N.is_power_of_two() {
            panic!("RingBuf size N must be a power of two");
        }
    }
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

use fixed::traits::Fixed;

#[derive(Copy, Clone)]
pub struct IirFilter<T> {
    y: T,
    alpha: T,
}

impl<T> IirFilter<T>
where
    T: Fixed + Copy, // Ensures T supports fixed-point math and is copyable
{
    pub const fn new(alpha: T) -> Self {
        Self {
            // We use from_bits(0) or simply T::ZERO if available
            y: T::ZERO,
            alpha,
        }
    }

    #[inline(always)]
    pub fn update(&mut self, x: T) -> T {
        // Standard EMA formula: y = y + alpha * (x - y)
        // Instead of: self.y += self.alpha * (x - self.y);
        // Use the built-in lerp which handles intermediate overflow better:
        // self.y = self.y.lerp(x, self.alpha);
        self.y += self.alpha.saturating_mul(x - self.y);
        self.y
    }
}
