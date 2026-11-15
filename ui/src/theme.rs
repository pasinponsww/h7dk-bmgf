//! The 70 / 20 / 10 color theme as RGB565 constants. No other file in the
//! project names a color.
//!
//! 70%: light pinks. 20%: pink and mauve. 10%: chrome, where gunmetal carries
//! the text and the silver gradient is a pre-rendered sprite (not a constant).

use embedded_graphics::pixelcolor::raw::RawU16;
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::RawData;

/// Converts 8-bit-per-channel RGB to the display's 16-bit format.
pub const fn rgb(r: u8, g: u8, b: u8) -> Rgb565 {
    Rgb565::new(r >> 3, g >> 2, b >> 3)
}

/// The 16-bit value of a color, as it is stored in the LCD's framebuffer.
pub fn to_raw(color: Rgb565) -> u16 {
    RawU16::from(color).into_inner()
}

/// 70%: screen background, very light pink `#FFF1F5`.
pub const BACKGROUND: Rgb565 = rgb(0xFF, 0xF1, 0xF5);
/// 70%: cards and soft surfaces, light pink `#FADDE1`.
pub const SURFACE: Rgb565 = rgb(0xFA, 0xDD, 0xE1);
/// 20%: tiles and big shapes, pink `#FFCAD4`.
pub const TILE: Rgb565 = rgb(0xFF, 0xCA, 0xD4);
/// 20%: secondary shapes, mauve `#EADBDD`.
pub const MAUVE: Rgb565 = rgb(0xEA, 0xDB, 0xDD);
/// 10% chrome: all text, gunmetal `#3B3F4A`.
pub const TEXT: Rgb565 = rgb(0x3B, 0x3F, 0x4A);
/// Icon color, a deeper tone of the tile pink `#A9506B`.
pub const ICON: Rgb565 = rgb(0xA9, 0x50, 0x6B);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colors_match_the_values_shown_in_the_plan() {
        assert_eq!(to_raw(BACKGROUND), 0xFF9E);
        assert_eq!(to_raw(SURFACE), 0xFEFC);
        assert_eq!(to_raw(TILE), 0xFE5A);
        assert_eq!(to_raw(MAUVE), 0xEEDB);
        assert_eq!(to_raw(TEXT), 0x39E9);
    }

    #[test]
    fn rgb_drops_the_low_bits_of_each_channel() {
        assert_eq!(to_raw(rgb(0, 0, 0)), 0x0000);
        assert_eq!(to_raw(rgb(255, 255, 255)), 0xFFFF);
        assert_eq!(to_raw(rgb(255, 0, 0)), 0xF800);
        assert_eq!(to_raw(rgb(0, 255, 0)), 0x07E0);
        assert_eq!(to_raw(rgb(0, 0, 255)), 0x001F);
    }
}
