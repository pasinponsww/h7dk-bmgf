//! The STM32H735's CORDIC co-processor, for sine and cosine.
//!
//! Set up once for the cosine function in 16-bit mode: one write carries the
//! angle (and a modulus of 1.0), and one read returns both the cosine and the
//! sine, the read waiting the few cycles the result takes. Angles are binary
//! turns (65536 = one turn) and results Q15, as in `ui::trig`.

use stm32h7xx_hal::pac;
use stm32h7xx_hal::rcc::rec;
use stm32h7xx_hal::rcc::ResetEnable as _;

/// A modulus of 1.0 in Q15, in the upper half of the argument word.
const MODULUS_ONE: u32 = 0x7FFF << 16;

/// The CORDIC.
pub struct Cordic {
    regs: pac::CORDIC,
}

impl Cordic {
    pub(crate) fn new(regs: pac::CORDIC, rec: rec::Cordic) -> Self {
        rec.enable().reset();
        regs.csr.write(|w| {
            w.func()
                .cosine()
                .precision()
                .iters16()
                .nargs()
                .num1()
                .nres()
                .num1()
                .argsize()
                .bits16()
                .ressize()
                .bits16()
        });
        Self { regs }
    }

    /// `(sin, cos)` of `angle` (65536 = one turn), in Q15.
    pub fn sin_cos(&mut self, angle: u16) -> (i16, i16) {
        // The angle as a Q15 fraction of half a turn: a u16 binary angle read
        // as an i16 is exactly that, from -1 (a half turn back) to 1.
        self.regs
            .wdata
            .write(|w| w.arg().bits(MODULUS_ONE | u32::from(angle)));
        let r = self.regs.rdata.read().res().bits();
        let [cos_lo, cos_hi, sin_lo, sin_hi] = r.to_le_bytes();
        (
            i16::from_le_bytes([sin_lo, sin_hi]),
            i16::from_le_bytes([cos_lo, cos_hi]),
        )
    }
}
