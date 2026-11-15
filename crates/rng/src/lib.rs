//! Logic that handles random numbers.
//! This crate is consume by the `geometry` crate, which uses it to pick
// random spots for the YES/NO buttons.
#![no_std]

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_seed_gives_same_sequence() {
        let mut a = XorShift32::new(42);
        let mut b = XorShift32::new(42);
        for _ in 0..10 {
            assert_eq!(a.next_u32(), b.next_u32());
        }
    }

    #[test]
    fn different_seeds_differ() {
        let mut a = XorShift32::new(1);
        let mut b = XorShift32::new(2);
        assert_ne!(a.next_u32(), b.next_u32());
    }

    #[test]
    fn zero_seed_does_not_stick() {
        let mut r = XorShift32::new(0);
        assert_ne!(r.next_u32(), 0);
        assert_ne!(r.next_u32(), 0);
    }
}
