//! Pixel art drawn from ASCII: one character per pixel (see
//! [`crate::theme::sprite_rgb`] for the colors; `.` is clear).
//!
//! An [`Art`] is a rectangle of characters. A [`Piece`] is art placed on a
//! larger canvas (a face or a hat on a dog). A [`Stamp`] says where a canvas
//! goes on the screen, how big its pixels are and whether it is mirrored, so
//! a dog walking left is the right-facing art flipped.

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::{DrawTarget, Point, Size};
use embedded_graphics::primitives::Rectangle;

use crate::theme;

/// A rectangle of pixel-art characters.
#[derive(Copy, Clone, Debug)]
pub struct Art {
    /// Width in art pixels.
    pub w: u8,
    /// Height in art pixels.
    pub h: u8,
    /// The rows, each `w` characters.
    pub rows: &'static [&'static str],
}

/// Art placed at `(x, y)` on a canvas.
#[derive(Copy, Clone, Debug)]
pub struct Piece {
    /// Column on the canvas.
    pub x: i8,
    /// Row on the canvas.
    pub y: i8,
    /// The art.
    pub art: Art,
}

/// Where a canvas goes: its top-left corner on the screen, the size of one
/// art pixel, and, if it is mirrored, the canvas width to mirror within.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Stamp {
    /// The canvas's top-left corner on the screen.
    pub origin: Point,
    /// Screen pixels per art pixel.
    pub scale: u32,
    /// `Some(width)` to mirror left-right within a canvas that many art
    /// pixels wide.
    pub flip: Option<i32>,
}

/// The display color of a sprite character, or `None` for a clear one.
pub fn color(c: u8) -> Option<Rgb565> {
    theme::sprite_rgb(c).map(|[r, g, b]| theme::rgb(r, g, b))
}

fn to_i32(v: usize) -> i32 {
    i32::try_from(v).unwrap_or(i32::MAX)
}

impl Stamp {
    /// A canvas at `origin` with `scale` pixels, not mirrored.
    pub const fn at(origin: Point, scale: u32) -> Self {
        Self {
            origin,
            scale,
            flip: None,
        }
    }

    /// Draws `art` at column `dx`, row `dy` of the canvas. Only the rows
    /// inside the target's bounding box are worked on, and each run of
    /// same-colored pixels is one rectangle fill.
    pub fn art<D>(&self, target: &mut D, art: &Art, dx: i32, dy: i32) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let s = i32::try_from(self.scale.max(1)).unwrap_or(1);
        let clip = target.bounding_box();
        let clip_top = clip.top_left.y;
        let clip_bottom = clip_top + i32::try_from(clip.size.height).unwrap_or(0);
        for (r, row) in art.rows.iter().enumerate() {
            let y = self.origin.y + (dy + to_i32(r)) * s;
            if y + s <= clip_top {
                continue;
            }
            if y >= clip_bottom {
                break;
            }
            let bytes = row.as_bytes();
            let mut c = 0;
            while c < bytes.len() {
                let ch = bytes[c];
                let start = c;
                while c < bytes.len() && bytes[c] == ch {
                    c += 1;
                }
                let Some(color) = color(ch) else {
                    continue;
                };
                // Canvas columns of the run, mirrored if asked.
                let (a, b) = (dx + to_i32(start), dx + to_i32(c) - 1);
                let (a, b) = match self.flip {
                    Some(w) => (w - 1 - b, w - 1 - a),
                    None => (a, b),
                };
                let x = self.origin.x + a * s;
                let width = u32::try_from((b - a + 1) * s).unwrap_or(0);
                let size = Size::new(width, self.scale.max(1));
                target.fill_solid(&Rectangle::new(Point::new(x, y), size), color)?;
            }
        }
        Ok(())
    }

    /// Draws a piece where it belongs on the canvas.
    pub fn piece<D>(&self, target: &mut D, p: &Piece) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        self.art(target, &p.art, i32::from(p.x), i32::from(p.y))
    }
}

#[cfg(test)]
mod tests {
    use std::vec;

    use super::*;
    use crate::frame::{Band, WIDTH};
    use crate::theme::to_raw;

    const ART: Art = Art {
        w: 3,
        h: 2,
        rows: &["KW.", "..H"],
    };

    fn px(buf: &[u16], x: usize, y: usize) -> u16 {
        buf[y * WIDTH + x]
    }

    #[test]
    fn art_is_drawn_in_squares_of_its_scale() {
        let mut buf = vec![0u16; WIDTH * 8];
        let stamp = Stamp::at(Point::new(10, 0), 2);
        stamp.art(&mut Band::new(&mut buf, 0), &ART, 0, 0).unwrap();
        let k = to_raw(color(b'K').unwrap());
        let w = to_raw(color(b'W').unwrap());
        let h = to_raw(color(b'H').unwrap());
        assert_eq!((px(&buf, 10, 0), px(&buf, 11, 1)), (k, k));
        assert_eq!((px(&buf, 12, 0), px(&buf, 13, 1)), (w, w));
        assert_eq!(px(&buf, 14, 0), 0, "clear stays clear");
        assert_eq!((px(&buf, 14, 2), px(&buf, 15, 3)), (h, h));
    }

    #[test]
    fn a_mirrored_canvas_draws_right_to_left() {
        let mut buf = vec![0u16; WIDTH * 4];
        let stamp = Stamp {
            origin: Point::new(0, 0),
            scale: 1,
            flip: Some(5),
        };
        stamp.art(&mut Band::new(&mut buf, 0), &ART, 1, 0).unwrap();
        // Canvas columns 1 and 2 (K, W) land on 3 and 2; row 1's H on 1.
        assert_eq!(px(&buf, 3, 0), to_raw(color(b'K').unwrap()));
        assert_eq!(px(&buf, 2, 0), to_raw(color(b'W').unwrap()));
        assert_eq!(px(&buf, 1, 1), to_raw(color(b'H').unwrap()));
    }

    #[test]
    fn only_the_band_rows_are_touched() {
        let mut buf = vec![0u16; WIDTH * 2];
        let stamp = Stamp::at(Point::new(0, 10), 3);
        stamp.art(&mut Band::new(&mut buf, 13), &ART, 0, 0).unwrap();
        // Art row 1 covers screen rows 13 to 15; the band holds 13 and 14.
        let h = to_raw(color(b'H').unwrap());
        assert_eq!((px(&buf, 6, 0), px(&buf, 8, 1)), (h, h));
        assert_eq!(px(&buf, 0, 0), 0, "row 0's K is above the band");
    }
}
