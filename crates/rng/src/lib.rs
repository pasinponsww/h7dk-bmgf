//! Logic that handles random numbers.
//! This crate is consume by the `geometry` crate, which uses it to pick 
// random spots for the YES/NO buttons.
#![no_std]
#![warn(missing_docs)]

/// A source of random numbers.
pub trait Rng {
    fn next_u32(&mut self) -> u32;
}

#[derive(Copy, Clone, Debug)]
pub struct XorShift32(u32);

impl XorShift32 {
    /// Creates a generator. A zero seed is replaced because xorshift
    /// would get stuck at zero.
    pub const fn new(seed: u32) -> Self {
        Self(if seed == 0 { 0x9E37_79B9 } else { seed })
    }
}

impl Rng for XorShift32 {
    fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }
}
