//! The sticker: a glossy rose heart with a white edge, shown on the LCD
//! controller's second layer.
//!
//! It is drawn once, at start-up, into a small buffer of ARGB4444 pixels (4
//! bits each of alpha, red, green and blue). The controller lays that layer
//! over the picture by itself, with the alpha giving soft edges, and moves it
//! by changing two window registers, so a moving sticker costs no drawing.

use embedded_graphics::prelude::Point;

use crate::heart;

/// Width and height of the sticker, in pixels.
pub const SIZE: usize = 48;
/// Pixels in the sticker.
pub const PIXELS: usize = SIZE * SIZE;

/// Each pixel is worked out from this many samples across and down, so its
/// alpha is the share of the 16 samples inside the shape: smooth edges.
const SUB: i32 = 4;

/// ARGB4444 from 4-bit alpha and 8-bit color.
const fn argb(a: u8, [r, g, b]: [u8; 3]) -> u16 {
    (a as u16 & 0xf) << 12 | (r as u16 >> 4) << 8 | (g as u16 >> 4) << 4 | b as u16 >> 4
}

/// How many of the 16 samples of pixel (x, y) are inside a heart of `size`
/// (in samples) centered on `center` (in samples).
fn covered(center: Point, size: i32, x: usize, y: usize) -> u8 {
    let (x0, y0) = (to_i32(x) * SUB, to_i32(y) * SUB);
    let mut n = 0;
    for sy in y0..y0 + SUB {
        let spans = heart::spans(center, size, sy);
        for sx in x0..x0 + SUB {
            if spans.iter().flatten().any(|&(a, b)| (a..=b).contains(&sx)) {
                n += 1;
            }
        }
    }
    n
}

fn to_i32(v: usize) -> i32 {
    i32::try_from(v).unwrap_or(i32::MAX)
}

/// 4-bit alpha from a count of 16 samples.
fn alpha(samples: u8) -> u8 {
    samples.min(15)
}

/// Draws the sticker into `out`, row by row: a white heart, a rose heart a
/// little smaller inside it, and a soft white shine on the upper left lobe.
pub fn render(out: &mut [u16; PIXELS]) {
    const WHITE: [u8; 3] = [0xFF, 0xFF, 0xFF];
    const ROSE: [u8; 3] = [0xE0, 0x45, 0x7A];
    const SHINE: [u8; 3] = [0xFF, 0xD6, 0xE2];
    // All in samples (4 per pixel). The heart is 2.1 times its size wide and
    // 1.8 times as tall, so a size of 86 fills the 192 samples with a small
    // margin; its center sits a little above the middle because the point
    // reaches further down than the lobes reach up.
    let mid = to_i32(SIZE) * SUB / 2;
    let center = Point::new(mid, mid - 4);
    let outer = 86;
    let inner = 70;
    // On the left lobe, up and to the left of its middle.
    let shine = Point::new(58, 62);
    for (i, px) in out.iter_mut().enumerate() {
        let (x, y) = (i % SIZE, i / SIZE);
        let edge = covered(center, outer, x, y);
        let body = covered(center, inner, x, y);
        // The shine: a small round spot, its own coverage worked out the same way.
        let mut glint = 0;
        for sy in 0..SUB {
            for sx in 0..SUB {
                let dx = to_i32(x) * SUB + sx - shine.x;
                let dy = to_i32(y) * SUB + sy - shine.y;
                if dx * dx * 4 + dy * dy * 9 <= (5 * SUB) * (5 * SUB) * 4 {
                    glint += 1;
                }
            }
        }
        *px = if body > 0 {
            // Inside the rose heart: blend toward white at its edge and at
            // the shine, fully opaque.
            let rose_share = u16::from(alpha(body));
            let shine_share = u16::from(alpha(glint)).min(rose_share);
            let mix = |w: u8, r: u8, s: u8| {
                let (w, r, s) = (u16::from(w), u16::from(r), u16::from(s));
                let base = (w * (15 - rose_share) + r * rose_share) / 15;
                let v = (base * (15 - shine_share) + s * shine_share) / 15;
                u8::try_from(v).unwrap_or(0xFF)
            };
            argb(
                15,
                [
                    mix(WHITE[0], ROSE[0], SHINE[0]),
                    mix(WHITE[1], ROSE[1], SHINE[1]),
                    mix(WHITE[2], ROSE[2], SHINE[2]),
                ],
            )
        } else {
            // The white edge, see-through beyond it.
            argb(alpha(edge), WHITE)
        };
    }
}

/// A smooth wave from -1000 to 1000 and back, once every `period` ms: a
/// sine, closely enough for moving things around.
pub fn wave(t_ms: u32, period: u32) -> i32 {
    let period = period.max(2);
    let half = period / 2;
    let p = t_ms % period;
    let (q, sign) = if p < half { (p, 1) } else { (p - half, -1) };
    // A parabola through 0, the peak and 0 again over half a period.
    let q = i64::from(q);
    let h = i64::from(half.max(1));
    let v = 4_000 * q * (h - q) / (h * h);
    sign * i32::try_from(v).unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sticker() -> [u16; PIXELS] {
        let mut out = [0; PIXELS];
        render(&mut out);
        out
    }

    fn a(px: u16) -> u16 {
        px >> 12
    }

    #[test]
    fn the_corners_are_see_through_and_the_middle_is_solid_rose() {
        let s = sticker();
        for (x, y) in [(0, 0), (SIZE - 1, 0), (0, SIZE - 1), (SIZE - 1, SIZE - 1)] {
            assert_eq!(a(s[y * SIZE + x]), 0, "corner ({x}, {y})");
        }
        let middle = s[(SIZE / 2) * SIZE + SIZE / 2 + 4];
        assert_eq!(middle, argb(15, [0xE0, 0x45, 0x7A]), "{middle:#06x}");
    }

    #[test]
    fn a_white_edge_runs_round_the_heart() {
        let s = sticker();
        // Walk in from the left along the middle row: see-through, then
        // opaque white, then rose.
        let row = &s[(SIZE / 2) * SIZE..(SIZE / 2 + 1) * SIZE];
        let first_solid = row.iter().position(|&p| a(p) == 15).unwrap();
        assert_eq!(row[first_solid] & 0x0fff, 0x0fff, "white first");
        assert!(
            row[first_solid..].iter().any(|&p| p & 0x0f00 == 0x0e00),
            "then rose"
        );
        assert!(
            row[..first_solid].iter().all(|&p| a(p) < 15),
            "soft outside"
        );
    }

    #[test]
    fn the_shine_is_lighter_than_the_rose() {
        let s = sticker();
        let shine = s[(62 / 4) * SIZE + 58 / 4];
        let green = |p: u16| (p >> 4) & 0xf;
        assert!(green(shine) > 0x4, "{shine:#06x}");
    }

    #[test]
    fn the_wave_swings_smoothly_between_minus_and_plus_a_thousand() {
        assert_eq!(wave(0, 1_000), 0);
        assert_eq!(wave(250, 1_000), 1_000);
        assert_eq!(wave(500, 1_000), 0);
        assert_eq!(wave(750, 1_000), -1_000);
        assert_eq!(wave(1_250, 1_000), 1_000, "repeats");
    }
}
