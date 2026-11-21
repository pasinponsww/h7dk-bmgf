//! Places to draw into.
//!
//! [`Band`] is a strip of whole screen rows held in RAM. The board draws each
//! frame one band at a time and copies every finished band into the LCD's
//! framebuffer, so the screen never shows a half-drawn frame (no flicker) and
//! only one small buffer is needed next to the framebuffer.
//!
//! [`Scaled`] blows every pixel up into a square, which turns the built-in
//! fonts into big titles.

use core::convert::Infallible;

use embedded_graphics::geometry::Dimensions;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::{DrawTarget, Point, Size};
use embedded_graphics::primitives::Rectangle;
use embedded_graphics::Pixel;

use crate::theme::to_raw;

/// Screen width in pixels.
pub const WIDTH: usize = 480;
/// Screen height in pixels.
pub const HEIGHT: usize = 272;

fn px(v: usize) -> i32 {
    i32::try_from(v).unwrap_or(i32::MAX)
}

fn len(v: i32) -> u32 {
    u32::try_from(v).unwrap_or(0)
}

/// A strip of whole rows of the screen, starting at row `top`.
///
/// Drawing uses screen coordinates. Anything outside the strip is dropped, and
/// [`Dimensions::bounding_box`] reports only the strip, so drawing code can skip
/// whatever does not touch it.
pub struct Band<'a> {
    buf: &'a mut [u16],
    top: i32,
    rows: i32,
}

impl<'a> Band<'a> {
    /// A band over `buf`, which holds `buf.len() / WIDTH` rows, starting at row `top`.
    pub fn new(buf: &'a mut [u16], top: usize) -> Self {
        let rows = px(buf.len() / WIDTH);
        Self {
            buf,
            top: px(top),
            rows,
        }
    }

    /// The pixels of the band, row by row, as stored in the framebuffer.
    pub fn pixels(&self) -> &[u16] {
        self.buf
    }

    fn index(&self, x: i32, y: i32) -> Option<usize> {
        if x < 0 || x >= px(WIDTH) || y < self.top || y >= self.top + self.rows {
            return None;
        }
        let row = usize::try_from(y - self.top).ok()?;
        let col = usize::try_from(x).ok()?;
        Some(row * WIDTH + col)
    }
}

impl Dimensions for Band<'_> {
    fn bounding_box(&self) -> Rectangle {
        Rectangle::new(
            Point::new(0, self.top),
            Size::new(len(px(WIDTH)), len(self.rows)),
        )
    }
}

impl DrawTarget for Band<'_> {
    type Color = Rgb565;
    type Error = Infallible;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), Infallible>
    where
        I: IntoIterator<Item = Pixel<Rgb565>>,
    {
        for Pixel(p, color) in pixels {
            if let Some(i) = self.index(p.x, p.y) {
                self.buf[i] = to_raw(color);
            }
        }
        Ok(())
    }

    fn fill_solid(&mut self, area: &Rectangle, color: Rgb565) -> Result<(), Infallible> {
        let area = area.intersection(&self.bounding_box());
        if area.is_zero_sized() {
            return Ok(());
        }
        let raw = to_raw(color);
        let x0 = area.top_left.x;
        let x1 = x0 + px(area.size.width as usize);
        let y0 = area.top_left.y;
        for y in y0..y0 + px(area.size.height as usize) {
            if let (Some(a), Some(b)) = (self.index(x0, y), self.index(x1 - 1, y)) {
                self.buf[a..=b].fill(raw);
            }
        }
        Ok(())
    }
}

/// Draws into `inner` with every pixel turned into a `scale` by `scale` square,
/// with this target's (0, 0) at `origin` on the inner target.
pub struct Scaled<'a, D> {
    inner: &'a mut D,
    origin: Point,
    scale: i32,
}

impl<'a, D> Scaled<'a, D>
where
    D: DrawTarget<Color = Rgb565>,
{
    /// Wraps `inner`. A `scale` below 1 is treated as 1.
    pub fn new(inner: &'a mut D, origin: Point, scale: u32) -> Self {
        Self {
            inner,
            origin,
            scale: i32::try_from(scale.max(1)).unwrap_or(1),
        }
    }
}

impl<D> Dimensions for Scaled<'_, D>
where
    D: DrawTarget<Color = Rgb565>,
{
    fn bounding_box(&self) -> Rectangle {
        let b = self.inner.bounding_box();
        let tl = b.top_left - self.origin;
        Rectangle::new(
            Point::new(tl.x.div_euclid(self.scale), tl.y.div_euclid(self.scale)),
            Size::new(
                b.size.width / len(self.scale) + 2,
                b.size.height / len(self.scale) + 2,
            ),
        )
    }
}

impl<D> DrawTarget for Scaled<'_, D>
where
    D: DrawTarget<Color = Rgb565>,
{
    type Color = Rgb565;
    type Error = D::Error;

    fn draw_iter<I>(&mut self, pixels: I) -> Result<(), D::Error>
    where
        I: IntoIterator<Item = Pixel<Rgb565>>,
    {
        let side = len(self.scale);
        for Pixel(p, color) in pixels {
            let at = self.origin + p * self.scale;
            self.inner
                .fill_solid(&Rectangle::new(at, Size::new(side, side)), color)?;
        }
        Ok(())
    }

    fn fill_solid(&mut self, area: &Rectangle, color: Rgb565) -> Result<(), D::Error> {
        let at = self.origin + area.top_left * self.scale;
        let size = Size::new(
            area.size.width * len(self.scale),
            area.size.height * len(self.scale),
        );
        self.inner.fill_solid(&Rectangle::new(at, size), color)
    }
}

#[cfg(test)]
mod tests {
    use std::vec;

    use embedded_graphics::prelude::RgbColor;

    use super::*;

    #[test]
    fn a_band_keeps_only_its_own_rows() {
        let mut buf = vec![0u16; WIDTH * 4];
        let mut band = Band::new(&mut buf, 10);
        assert_eq!(
            band.bounding_box(),
            Rectangle::new(Point::new(0, 10), Size::new(480, 4))
        );
        band.draw_iter([
            Pixel(Point::new(5, 9), Rgb565::WHITE),
            Pixel(Point::new(5, 10), Rgb565::WHITE),
            Pixel(Point::new(479, 13), Rgb565::WHITE),
            Pixel(Point::new(480, 13), Rgb565::WHITE),
            Pixel(Point::new(0, 14), Rgb565::WHITE),
        ])
        .unwrap();
        let lit: std::vec::Vec<usize> = (0..buf.len()).filter(|i| buf[*i] != 0).collect();
        assert_eq!(lit, vec![5, 3 * WIDTH + 479]);
    }

    #[test]
    fn fill_solid_clips_to_the_band_and_the_screen() {
        let mut buf = vec![0u16; WIDTH * 2];
        let mut band = Band::new(&mut buf, 100);
        band.fill_solid(
            &Rectangle::new(Point::new(470, 90), Size::new(40, 100)),
            Rgb565::WHITE,
        )
        .unwrap();
        for row in 0..2 {
            for x in 0..WIDTH {
                let want = if x >= 470 { 0xFFFF } else { 0 };
                assert_eq!(buf[row * WIDTH + x], want, "({x}, {row})");
            }
        }
    }

    #[test]
    fn scaled_draws_squares() {
        let mut buf = vec![0u16; WIDTH * 8];
        let mut band = Band::new(&mut buf, 0);
        let mut big = Scaled::new(&mut band, Point::new(10, 2), 3);
        big.draw_iter([Pixel(Point::new(1, 1), Rgb565::WHITE)])
            .unwrap();
        for y in 0..8 {
            for x in 0..WIDTH {
                let inside = (13..16).contains(&x) && (5..8).contains(&y);
                assert_eq!(buf[y * WIDTH + x] != 0, inside, "({x}, {y})");
            }
        }
    }
}
