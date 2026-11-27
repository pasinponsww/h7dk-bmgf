//! The pixel fonts in `assets/fonts/pixel`, baked by `build.rs` into
//! embedded-graphics fonts: Press Start 2P (8 x 8, chunky 8-bit capitals and
//! all) and VT323 (8 x 20, a tall terminal font).

use embedded_graphics::geometry::Size;
use embedded_graphics::image::ImageRaw;
use embedded_graphics::mono_font::{mapping, DecorationDimensions, MonoFont};

include!(concat!(env!("OUT_DIR"), "/fonts.rs"));
