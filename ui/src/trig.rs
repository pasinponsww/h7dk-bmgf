//! Sine and cosine for turning things.
//!
//! Angles are binary turns: 65536 is one whole turn, so a `u16` wraps
//! around by itself. Results are Q15: 32767 is 1.0 and -32768 is -1.0.
//!
//! On the board the STM32H735's CORDIC co-processor works them out (see the
//! `bsp` crate); [`SoftTrig`] does the same in software for the PC tests and
//! anywhere the hardware is not at hand.

/// Something that knows sine and cosine.
pub trait Trig {
    /// `(sin, cos)` of `angle` (65536 = one turn), in Q15.
    fn sin_cos(&mut self, angle: u16) -> (i16, i16);
}

/// Sine and cosine in software, to within 0.2%.
#[derive(Copy, Clone, Debug, Default)]
pub struct SoftTrig;

/// Half a turn.
const HALF: i64 = 32_768;

/// Sine in Q15 by Bhaskara I's formula on each half turn:
/// sin x = 16 x (pi - x) / (5 pi^2 - 4 x (pi - x)), with pi = half a turn.
fn sin_q15(angle: u16) -> i16 {
    let a = i64::from(angle);
    let (t, sign) = if a < HALF { (a, 1) } else { (a - HALF, -1) };
    let p = t * (HALF - t);
    let v = 16 * p * 32_767 / (5 * HALF * HALF - 4 * p);
    i16::try_from(sign * v).unwrap_or(0)
}

impl Trig for SoftTrig {
    fn sin_cos(&mut self, angle: u16) -> (i16, i16) {
        (sin_q15(angle), sin_q15(angle.wrapping_add(16_384)))
    }
}

/// Turns `(x, y)` by the angle whose `(sin, cos)` is given, in Q15.
pub fn rotate(x: i32, y: i32, (sin, cos): (i16, i16)) -> (i32, i32) {
    let (s, c) = (i32::from(sin), i32::from(cos));
    ((x * c - y * s) >> 15, (x * s + y * c) >> 15)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn soft_trig_is_close_to_the_real_thing() {
        let mut t = SoftTrig;
        for a in (0..=65_535u32).step_by(97) {
            let angle = u16::try_from(a).unwrap();
            let (s, c) = t.sin_cos(angle);
            let x = f64::from(a) / 65_536.0 * core::f64::consts::TAU;
            let (rs, rc) = (x.sin() * 32_767.0, x.cos() * 32_767.0);
            assert!((f64::from(s) - rs).abs() < 70.0, "sin at {a}: {s} vs {rs}");
            assert!((f64::from(c) - rc).abs() < 70.0, "cos at {a}: {c} vs {rc}");
        }
    }

    #[test]
    fn the_quarter_turns_are_exact() {
        let mut t = SoftTrig;
        assert_eq!(t.sin_cos(0), (0, 32_767));
        assert_eq!(t.sin_cos(16_384), (32_767, 0));
        assert_eq!(t.sin_cos(32_768), (0, -32_767));
        assert_eq!(t.sin_cos(49_152), (-32_767, 0));
    }

    #[test]
    fn rotating_a_quarter_turn_swaps_the_axes() {
        let quarter = SoftTrig.sin_cos(16_384);
        assert_eq!(rotate(100, 0, quarter), (0, 99));
        assert_eq!(rotate(0, 100, quarter), (-100, 0));
        assert_eq!(rotate(37, -12, SoftTrig.sin_cos(0)), (36, -12));
    }
}
