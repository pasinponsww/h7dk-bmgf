//! Photos, baked by the app's `build.rs` from the files a story names: cut
//! to a square, shrunk, and dithered to the screen's 16-bit colors.
//!
//! A photo is stored as RGB565 pixels, two bytes each (low byte first), row
//! by row, in flash. Drawing reads them straight from there, so a photo uses
//! no RAM.

use embedded_graphics::pixelcolor::raw::RawU16;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::{DrawTarget, Point};
use embedded_graphics::Pixel;

use crate::icon::mix;

/// What an instant photo looks like before it develops: a creamy grey.
const UNDEVELOPED: Rgb565 = crate::theme::rgb(0xE8, 0xE4, 0xDA);

/// A square photo.
#[derive(Copy, Clone, Debug)]
pub struct Photo {
    /// Width and height in pixels.
    pub size: u16,
    /// `size * size` RGB565 pixels, low byte first.
    pub px: &'static [u8],
}

impl Photo {
    /// A photo `size` pixels square over `px`, which must hold `size * size * 2` bytes.
    pub const fn new(size: u16, px: &'static [u8]) -> Self {
        Self { size, px }
    }

    /// The color of pixel `i` (row by row).
    pub fn pixel(&self, i: usize) -> Rgb565 {
        let lo = self.px.get(2 * i).copied().unwrap_or(0);
        let hi = self.px.get(2 * i + 1).copied().unwrap_or(0);
        Rgb565::from(RawU16::new(u16::from_le_bytes([lo, hi])))
    }

    /// Draws the photo with its top-left corner at `at`, `developed`
    /// fifteenths of the way from a blank instant photo to the real colors.
    /// Only the rows inside the target's bounding box are worked on.
    pub fn draw<D>(&self, target: &mut D, at: Point, developed: u8) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let size = usize::from(self.size);
        if size == 0 || self.px.len() < size * size * 2 {
            return Ok(());
        }
        let clip = target.bounding_box();
        let clip_top = clip.top_left.y;
        let clip_bottom = clip_top + i32::try_from(clip.size.height).unwrap_or(0);
        let developed = developed.min(15);
        for y in 0..size {
            let py = at.y + i32::try_from(y).unwrap_or(i32::MAX);
            if py < clip_top {
                continue;
            }
            if py >= clip_bottom {
                break;
            }
            let row = (0..size).map(|x| {
                let color = self.pixel(y * size + x);
                let color = if developed == 15 {
                    color
                } else {
                    mix(UNDEVELOPED, color, developed)
                };
                Pixel(Point::new(at.x + i32::try_from(x).unwrap_or(0), py), color)
            });
            target.draw_iter(row)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::vec;

    use embedded_graphics::pixelcolor::RgbColor;

    use super::*;
    use crate::frame::{Band, WIDTH};
    use crate::theme::to_raw;

    /// A 2 x 2 photo: red, green / blue, white.
    static PX: [u8; 8] = [0x00, 0xF8, 0xE0, 0x07, 0x1F, 0x00, 0xFF, 0xFF];

    #[test]
    fn pixels_are_read_low_byte_first() {
        let p = Photo::new(2, &PX);
        assert_eq!(p.pixel(0), Rgb565::RED);
        assert_eq!(p.pixel(1), Rgb565::GREEN);
        assert_eq!(p.pixel(2), Rgb565::BLUE);
        assert_eq!(p.pixel(3), Rgb565::WHITE);
        assert_eq!(p.pixel(99), Rgb565::BLACK, "out of range is safe");
    }

    #[test]
    fn a_developed_photo_is_drawn_as_it_is_and_clipped_to_the_band() {
        let p = Photo::new(2, &PX);
        let mut buf = vec![0u16; WIDTH * 2];
        p.draw(&mut Band::new(&mut buf, 5), Point::new(10, 4), 15)
            .unwrap();
        // Row 4 is above the band, row 5 is the photo's second row.
        assert_eq!(buf[10], to_raw(Rgb565::BLUE));
        assert_eq!(buf[11], to_raw(Rgb565::WHITE));
        assert_eq!(buf.iter().filter(|&&v| v != 0).count(), 2);
    }

    #[test]
    fn an_undeveloped_photo_is_a_blank_cream() {
        let p = Photo::new(2, &PX);
        let mut buf = vec![0u16; WIDTH * 2];
        p.draw(&mut Band::new(&mut buf, 0), Point::new(0, 0), 0)
            .unwrap();
        assert!(buf[..2].iter().all(|&v| v == to_raw(UNDEVELOPED)));
        let mut half = vec![0u16; WIDTH * 2];
        p.draw(&mut Band::new(&mut half, 0), Point::new(0, 0), 8)
            .unwrap();
        assert_ne!(half[0], to_raw(UNDEVELOPED));
        assert_ne!(half[0], to_raw(Rgb565::RED), "half way there");
    }
}
