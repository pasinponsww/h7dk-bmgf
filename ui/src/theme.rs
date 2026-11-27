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

/// [`BACKGROUND`] in 8 bits per channel, for the LCD controller's own
/// background color (what the screen fades to).
pub const BACKGROUND_RGB: [u8; 3] = [0xFF, 0xF1, 0xF5];
/// 70%: screen background, very light pink `#FFF1F5`.
pub const BACKGROUND: Rgb565 = rgb(BACKGROUND_RGB[0], BACKGROUND_RGB[1], BACKGROUND_RGB[2]);
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
/// The soft fill inside a found object's icon, rose pink `#F5A9BD`.
pub const ICON_FILL: Rgb565 = rgb(0xF5, 0xA9, 0xBD);
/// The close, darker half of the soft shadow under tiles and buttons, `#EFC9D4`.
pub const SHADOW: Rgb565 = rgb(0xEF, 0xC9, 0xD4);
/// The far, lighter half of that shadow, `#F7DDE5`.
pub const SHADOW_SOFT: Rgb565 = rgb(0xF7, 0xDD, 0xE5);
/// The white of a polaroid's frame, a little warm, `#FFFBF0` (kept apart
/// from the dog's pure white fur, which 16-bit color would otherwise merge
/// it with).
pub const POLAROID: Rgb565 = rgb(0xFF, 0xFB, 0xF0);
/// The dog's sweat drops on a hot board, sky blue `#8CCBEA`.
pub const SWEAT: Rgb565 = rgb(0x8C, 0xCB, 0xEA);
/// Quiet text: hints and buttons that do not work yet, `#8C858E`.
pub const MUTED: Rgb565 = rgb(0x8C, 0x85, 0x8E);
/// Text on an [`ICON`]-colored button: the background pink.
pub const ON_ICON: Rgb565 = BACKGROUND;
/// The floating hearts and the dog's collar, rose `#E0457A`.
pub const HEART: Rgb565 = rgb(0xE0, 0x45, 0x7A);

/// 8-bit RGB of a mascot sprite character, or `None` for a transparent cell.
///
/// K outline, W white fur, S fur shade, D deep shade and collar tag, P inner
/// ear, Q inner ear shade, E eye, N nose, T tongue, B blush, H heart and collar.
pub const fn mascot_rgb(c: u8) -> Option<[u8; 3]> {
    match c {
        b'K' => Some([0x3B, 0x3F, 0x4A]),
        b'W' => Some([0xFF, 0xFF, 0xFF]),
        b'S' => Some([0xE2, 0xE5, 0xEE]),
        b'D' => Some([0xC3, 0xC9, 0xD6]),
        b'P' => Some([0xFF, 0xB6, 0xC8]),
        b'Q' => Some([0xF0, 0x98, 0xB0]),
        b'E' => Some([0x23, 0x25, 0x2E]),
        b'N' => Some([0x2B, 0x2D, 0x36]),
        b'T' => Some([0xE5, 0x70, 0x8F]),
        b'B' => Some([0xFF, 0xA6, 0xBA]),
        b'H' => Some([0xE0, 0x45, 0x7A]),
        _ => None,
    }
}

/// The display color of a mascot sprite character, or `None` for a transparent cell.
pub fn mascot_color(c: u8) -> Option<Rgb565> {
    mascot_rgb(c).map(|[r, g, b]| rgb(r, g, b))
}

/// 8-bit RGB of the backdrop glyphs at brightness `level` 1 to 3: pinks that sit
/// just above [`BACKGROUND`], so the moving texture stays quiet behind the text.
pub const fn backdrop_rgb(level: u8) -> [u8; 3] {
    match level {
        0 | 1 => [0xFA, 0xDF, 0xE6],
        2 => [0xF5, 0xC9, 0xD5],
        _ => [0xEC, 0xAE, 0xC0],
    }
}

/// The display color of a backdrop glyph at brightness `level`.
pub fn backdrop_color(level: u8) -> Rgb565 {
    let [r, g, b] = backdrop_rgb(level);
    rgb(r, g, b)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backdrop_levels_get_darker_but_stay_light() {
        let lum = |[r, g, b]: [u8; 3]| u32::from(r) + u32::from(g) + u32::from(b);
        assert!(lum(backdrop_rgb(1)) > lum(backdrop_rgb(2)));
        assert!(lum(backdrop_rgb(2)) > lum(backdrop_rgb(3)));
        assert!(
            lum(backdrop_rgb(3)) > 500,
            "level 3 must still read as pastel"
        );
        assert_eq!(backdrop_rgb(0), backdrop_rgb(1));
        assert_eq!(backdrop_rgb(9), backdrop_rgb(3));
    }

    #[test]
    fn mascot_characters_have_colors_and_dots_are_transparent() {
        for c in *b"KWSDPQENTBH" {
            assert!(mascot_color(c).is_some(), "{}", char::from(c));
        }
        assert_eq!(mascot_color(b'.'), None);
        assert_eq!(mascot_color(b'?'), None);
        assert_eq!(to_raw(mascot_color(b'W').unwrap()), 0xFFFF);
    }

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
