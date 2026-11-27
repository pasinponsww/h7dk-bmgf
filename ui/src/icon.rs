//! Two-tone icons, baked from the Phosphor duotone SVGs when the firmware is
//! built (see the app's `build.rs`).
//!
//! An icon is a square of bytes, one per pixel, row by row. The high nibble
//! says how much the outline covers the pixel (0 = not at all, 15 = fully),
//! the low nibble how much the soft fill behind it does. Drawing blends the
//! fill color and then the outline color over a known background color, so
//! edges stay smooth without reading the screen back.

use embedded_graphics::pixelcolor::{Rgb565, RgbColor};
use embedded_graphics::prelude::{DrawTarget, Point};
use embedded_graphics::Pixel;

/// A square two-tone icon.
#[derive(Copy, Clone, Debug)]
pub struct Icon {
    /// Width and height in pixels.
    pub size: u16,
    /// `size * size` bytes: outline coverage in the high nibble, fill in the low.
    pub px: &'static [u8],
}

/// The three colors an icon is drawn with.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Tones {
    /// What the icon sits on.
    pub back: Rgb565,
    /// The soft shape behind the outline.
    pub fill: Rgb565,
    /// The outline.
    pub line: Rgb565,
}

/// `a` moved `t` fifteenths of the way to `b`.
pub fn mix(a: Rgb565, b: Rgb565, t: u8) -> Rgb565 {
    let t = i32::from(t.min(15));
    let lerp = |x: u8, y: u8| {
        let (x, y) = (i32::from(x), i32::from(y));
        u8::try_from(x + (y - x) * t / 15).unwrap_or(0)
    };
    Rgb565::new(lerp(a.r(), b.r()), lerp(a.g(), b.g()), lerp(a.b(), b.b()))
}

impl Icon {
    /// An icon `size` pixels square over `px`, which must hold `size * size` bytes.
    pub const fn new(size: u16, px: &'static [u8]) -> Self {
        Self { size, px }
    }

    /// The color of one pixel byte, or `None` where the icon is empty.
    pub fn shade(byte: u8, tones: Tones) -> Option<Rgb565> {
        if byte == 0 {
            return None;
        }
        let under = mix(tones.back, tones.fill, byte & 0x0f);
        Some(mix(under, tones.line, byte >> 4))
    }

    /// Draws the icon with its top-left corner at `at`. Only the rows inside
    /// the target's bounding box are worked on.
    pub fn draw<D>(&self, target: &mut D, at: Point, tones: Tones) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let size = usize::from(self.size);
        if size == 0 || self.px.len() < size * size {
            return Ok(());
        }
        let clip = target.bounding_box();
        let clip_top = clip.top_left.y;
        let clip_bottom = clip_top + i32::try_from(clip.size.height).unwrap_or(0);
        for (y, row) in self.px.chunks_exact(size).enumerate() {
            let py = at.y + i32::try_from(y).unwrap_or(i32::MAX);
            if py < clip_top {
                continue;
            }
            if py >= clip_bottom {
                break;
            }
            let pixels = row.iter().enumerate().filter_map(|(x, &b)| {
                let color = Self::shade(b, tones)?;
                let px = at.x + i32::try_from(x).unwrap_or(i32::MAX);
                Some(Pixel(Point::new(px, py), color))
            });
            target.draw_iter(pixels)?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::vec;

    use embedded_graphics::prelude::RgbColor;

    use super::{mix, Icon, Tones};
    use crate::frame::{Band, WIDTH};
    use crate::theme::to_raw;
    use embedded_graphics::pixelcolor::Rgb565;
    use embedded_graphics::prelude::Point;

    const TONES: Tones = Tones {
        back: Rgb565::WHITE,
        fill: Rgb565::RED,
        line: Rgb565::BLUE,
    };

    #[test]
    fn mix_goes_from_one_color_to_the_other() {
        assert_eq!(mix(Rgb565::BLACK, Rgb565::WHITE, 0), Rgb565::BLACK);
        assert_eq!(mix(Rgb565::BLACK, Rgb565::WHITE, 15), Rgb565::WHITE);
        assert_eq!(mix(Rgb565::BLACK, Rgb565::WHITE, 99), Rgb565::WHITE);
        let half = mix(Rgb565::BLACK, Rgb565::WHITE, 7);
        assert!(half.g() > 20 && half.g() < 40, "about half way: {half:?}");
    }

    #[test]
    fn shade_layers_the_outline_over_the_fill() {
        assert_eq!(Icon::shade(0x00, TONES), None, "empty stays see-through");
        assert_eq!(Icon::shade(0x0f, TONES), Some(Rgb565::RED), "fill only");
        assert_eq!(Icon::shade(0xf0, TONES), Some(Rgb565::BLUE), "outline only");
        assert_eq!(Icon::shade(0xff, TONES), Some(Rgb565::BLUE), "outline wins");
    }

    #[test]
    fn draw_puts_pixels_where_the_icon_has_them_and_clips_to_the_band() {
        static PX: [u8; 9] = [0xf0, 0, 0, 0, 0x0f, 0, 0, 0, 0xf0];
        let icon = Icon::new(3, &PX);
        let mut buf = vec![0u16; WIDTH * 2];
        let mut band = Band::new(&mut buf, 11);
        icon.draw(&mut band, Point::new(5, 10), TONES).unwrap();
        // Row 10 is above the band; row 11 is the icon's middle row.
        assert_eq!(buf[6], to_raw(Rgb565::RED));
        assert_eq!(buf[5], 0);
        assert_eq!(buf[7], 0);
        // Row 12 is the icon's last row.
        assert_eq!(buf[WIDTH + 7], to_raw(Rgb565::BLUE));
        assert_eq!(buf.iter().filter(|&&p| p != 0).count(), 2);
    }

    #[test]
    fn a_short_pixel_slice_draws_nothing() {
        static PX: [u8; 3] = [0xff; 3];
        let mut buf = vec![0u16; WIDTH * 4];
        let mut band = Band::new(&mut buf, 0);
        Icon::new(2, &PX)
            .draw(&mut band, Point::new(0, 0), TONES)
            .unwrap();
        assert!(buf.iter().all(|&p| p == 0));
    }
}
