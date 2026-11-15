//! The moving background: a soft, slow rain of small ASCII glyphs behind every
//! scene, instead of a flat pink.
//!
//! Nothing is stored between frames. [`cell`] answers "what is in this cell at
//! this time" from a hash of the column, the row and the time, so a frame costs
//! one hash per cell and the board needs no buffer for it. Each column falls at
//! its own speed and some glyphs twinkle, which is enough to feel alive.

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::{DrawTarget, Point, Size};
use embedded_graphics::primitives::Rectangle;

use crate::theme;

/// Size of one cell on the 480 x 272 screen, in pixels.
pub const CELL_PX: i32 = 8;
/// Cells across the screen.
pub const COLS: usize = 60;
/// Cells down the screen.
pub const ROWS: usize = 34;
/// Cells on the screen.
pub const CELLS: usize = COLS * ROWS;

/// The characters that can appear, from quietest to busiest.
pub const GLYPHS: [u8; 6] = *b".:'+*o";

/// Roughly one cell in this many holds a glyph.
const SPARSITY: u32 = 9;
/// Height of the invisible strip that scrolls past, in cells. A prime, so the
/// pattern repeats late.
const STRIP: u32 = 97;
/// Slowest fall: one cell per this many milliseconds.
const SLOWEST_MS: u32 = 1_500;
/// Each faster column speed takes this much less time per cell.
const SPEED_STEP_MS: u32 = 220;

/// What is in one cell.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Glyph {
    /// The character, or a space for nothing.
    pub ch: u8,
    /// Brightness 1 to 3, or 0 for nothing.
    pub level: u8,
}

/// An empty cell.
pub const EMPTY: Glyph = Glyph { ch: b' ', level: 0 };

fn hash(mut v: u32) -> u32 {
    v ^= v >> 16;
    v = v.wrapping_mul(0x7feb_352d);
    v ^= v >> 15;
    v = v.wrapping_mul(0x846c_a68b);
    v ^= v >> 16;
    v
}

fn index(v: usize) -> u32 {
    u32::try_from(v).unwrap_or(0)
}

/// Which glyph sits in column `x`, row `y` at `t_ms` milliseconds.
pub fn cell(x: usize, y: usize, t_ms: u32) -> Glyph {
    let column = hash(index(x).wrapping_mul(0x9e37_79b9).wrapping_add(7));
    let period = SLOWEST_MS - (column % 5) * SPEED_STEP_MS;
    let fallen = t_ms / period;
    let row = (index(y) + STRIP * 1_000 - fallen % (STRIP * 1_000)) % STRIP;
    let k = hash(column ^ row.wrapping_mul(0x85eb_ca6b));
    if !k.is_multiple_of(SPARSITY) {
        return EMPTY;
    }
    let pick = k / SPARSITY;
    let ch = GLYPHS[(pick % index(GLYPHS.len())) as usize];
    let base = 1 + u8::try_from(pick / 7 % 3).unwrap_or(0);
    let twinkles = (pick / 21).is_multiple_of(3);
    let bright = twinkles && (t_ms / 250 + pick) % 6 < 2;
    Glyph {
        ch,
        level: if bright { (base + 1).min(3) } else { base },
    }
}

/// Fills `out` with one byte per cell, row by row: 0 for nothing, otherwise
/// `glyph number + 1` in the low bits and the level in the high bits.
pub fn render(t_ms: u32, out: &mut [u8; CELLS]) {
    for (i, slot) in out.iter_mut().enumerate() {
        let g = cell(i % COLS, i / COLS, t_ms);
        *slot = if g.level == 0 {
            0
        } else {
            let n = GLYPHS.iter().position(|c| *c == g.ch).unwrap_or(0);
            u8::try_from(n + 1).unwrap_or(1) | g.level << 4
        };
    }
}

/// The 5 x 5 pixel mark the board draws for a glyph, when no font is loaded.
/// One row per entry, bit 4 is the left pixel.
pub const fn mark(ch: u8) -> [u8; 5] {
    match ch {
        b'.' => [0b00000, 0b00000, 0b00100, 0b00000, 0b00000],
        b':' => [0b00000, 0b00100, 0b00000, 0b00100, 0b00000],
        b'\'' => [0b00100, 0b00100, 0b00000, 0b00000, 0b00000],
        b'+' => [0b00000, 0b00100, 0b01110, 0b00100, 0b00000],
        b'*' => [0b00100, 0b10101, 0b01110, 0b10101, 0b00100],
        b'o' => [0b00000, 0b01110, 0b01010, 0b01110, 0b00000],
        _ => [0; 5],
    }
}

/// Paints the glyphs at `t_ms` over the whole screen area starting at `origin`.
/// Call it after filling the background and before drawing the scene.
pub fn draw<D>(target: &mut D, origin: Point, t_ms: u32) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    let px = Size::new(1, 1);
    for y in 0..ROWS {
        for x in 0..COLS {
            let g = cell(x, y, t_ms);
            if g.level == 0 {
                continue;
            }
            let color = theme::backdrop_color(g.level);
            let base = origin
                + Point::new(
                    i32::try_from(x).unwrap_or(0) * CELL_PX + 1,
                    i32::try_from(y).unwrap_or(0) * CELL_PX + 1,
                );
            for (row, bits) in mark(g.ch).iter().enumerate() {
                for col in 0..5 {
                    if bits & (0b10000 >> col) != 0 {
                        let at = base + Point::new(col, i32::try_from(row).unwrap_or(0));
                        target.fill_solid(&Rectangle::new(at, px), color)?;
                    }
                }
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn frame(t: u32) -> [u8; CELLS] {
        let mut out = [0; CELLS];
        render(t, &mut out);
        out
    }

    fn filled(f: &[u8; CELLS]) -> usize {
        f.iter().filter(|b| **b != 0).count()
    }

    #[test]
    fn the_grid_covers_the_screen() {
        assert_eq!(COLS as i32 * CELL_PX, 480);
        assert_eq!(ROWS as i32 * CELL_PX, 272);
    }

    #[test]
    fn the_same_moment_always_looks_the_same() {
        assert_eq!(frame(1_234), frame(1_234));
        assert_eq!(cell(3, 4, 500), cell(3, 4, 500));
    }

    #[test]
    fn it_is_sparse_enough_to_read_text_over() {
        for t in [0, 777, 5_000, 60_000, 3_600_000] {
            let n = filled(&frame(t));
            let pct = n * 100 / CELLS;
            assert!(
                (5..=20).contains(&pct),
                "{pct}% of cells are filled at {t} ms"
            );
        }
    }

    #[test]
    fn every_glyph_is_known_and_every_level_is_one_to_three() {
        let f = frame(42_000);
        for b in f {
            if b != 0 {
                let n = usize::from(b & 0x0f);
                assert!((1..=GLYPHS.len()).contains(&n));
                assert!((1..=3).contains(&(b >> 4)));
            }
        }
        for y in 0..ROWS {
            for x in 0..COLS {
                let g = cell(x, y, 42_000);
                assert_eq!(g.level == 0, g.ch == b' ');
                assert!(g.level == 0 || GLYPHS.contains(&g.ch));
            }
        }
    }

    #[test]
    fn it_moves_but_not_all_at_once() {
        let a = frame(0);
        let b = frame(400);
        let c = frame(4_000);
        assert_ne!(a, c, "nothing changed in four seconds");
        let changed = a.iter().zip(b.iter()).filter(|(x, y)| x != y).count();
        assert!(
            changed < CELLS / 4,
            "{changed} cells changed in 0.4 s, that flickers"
        );
    }

    #[test]
    fn columns_fall_at_different_speeds() {
        let speed = |x: usize| {
            (0..20_000u32)
                .step_by(50)
                .filter(|t| cell(x, 10, *t) != cell(x, 10, t + 50))
                .count()
        };
        let speeds: std::vec::Vec<usize> = (0..COLS).map(speed).collect();
        assert!(speeds.iter().max() > speeds.iter().min());
    }

    #[test]
    fn some_glyphs_twinkle() {
        let mut saw = false;
        'outer: for y in 0..ROWS {
            for x in 0..COLS {
                let a = cell(x, y, 10_000);
                if a.level == 0 {
                    continue;
                }
                for dt in (0..1_500).step_by(50) {
                    let b = cell(x, y, 10_000 + dt);
                    if b.ch == a.ch && b.level != a.level {
                        saw = true;
                        break 'outer;
                    }
                }
            }
        }
        assert!(saw);
    }

    #[test]
    fn every_glyph_has_a_mark_and_blanks_do_not() {
        for ch in GLYPHS {
            assert!(mark(ch).iter().any(|r| *r != 0), "{}", char::from(ch));
            assert!(mark(ch).iter().all(|r| *r < 32));
        }
        assert_eq!(mark(b' '), [0; 5]);
    }
}
