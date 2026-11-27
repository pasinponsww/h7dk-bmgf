//! The places a story moment can happen in: a whole-screen background for
//! each, drawn from rectangles and circles (and a few sprites) so a frame
//! stays cheap, with a little movement of its own: waves, falling leaves,
//! a flickering cinema screen, twinkling stars and windows.
//!
//! Everything is worked out from the time, so nothing is stored between
//! frames. The floor where the dogs walk starts at [`FLOOR_Y`].

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::{DrawTarget, Point, Primitive, Size};
use embedded_graphics::primitives::{Circle, PrimitiveStyle, Rectangle, Triangle};
use embedded_graphics::Drawable;

use crate::art;
use crate::sprite::Stamp;
use crate::theme::rgb;

/// Where the floor (or sand, or street) begins.
pub const FLOOR_Y: i32 = 150;
const W: i32 = 480;
const H: i32 = 272;

/// A place a moment happens in.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Place {
    /// The usual pink with drifting glyphs (drawn by the app).
    Pink,
    /// A club room: warm walls, a door and a wooden floor.
    Club,
    /// The beach: sky, rolling sea and sand.
    Beach,
    /// Autumn: trees in orange and red, leaves falling.
    Fall,
    /// A dark cinema with a glowing screen and rows of seats.
    Cinema,
    /// A city street at dusk, windows lit.
    City,
    /// A gallery with paintings on white walls.
    Museum,
    /// A night street: moon, stars and a street lamp.
    Night,
    /// A cozy café: a window, shelves of cups and a hanging lamp.
    Cafe,
    /// Dreamy: pink into lavender, white stars twinkling.
    Dream,
}

impl Place {
    /// Every place, in the order of [`Place::NAMES`].
    pub const ALL: [Place; 10] = [
        Place::Pink,
        Place::Club,
        Place::Beach,
        Place::Fall,
        Place::Cinema,
        Place::City,
        Place::Museum,
        Place::Night,
        Place::Cafe,
        Place::Dream,
    ];
    /// The names a story file may use.
    pub const NAMES: [&'static str; 10] = [
        "pink", "club", "beach", "fall", "cinema", "city", "museum", "night", "cafe", "dream",
    ];

    /// The place called `name`, if there is one.
    pub fn from_name(name: &str) -> Option<Place> {
        Self::NAMES
            .iter()
            .position(|n| *n == name)
            .map(|i| Self::ALL[i])
    }

    /// True for the dark places, where text needs a light plate under it.
    pub fn is_dark(self) -> bool {
        matches!(self, Place::Cinema | Place::Night)
    }

    /// The color of a shadow on this place's floor.
    pub fn shadow(self) -> Rgb565 {
        match self {
            Place::Pink => rgb(0xE8, 0xBA, 0xCA),
            Place::Club => rgb(0xC2, 0x94, 0x68),
            Place::Beach => rgb(0xDC, 0xBF, 0x84),
            Place::Fall => rgb(0xB8, 0x8C, 0x58),
            Place::Cinema => rgb(0x26, 0x1D, 0x38),
            Place::City => rgb(0xA8, 0xA0, 0xBC),
            Place::Museum => rgb(0xCC, 0xB8, 0x9C),
            Place::Night => rgb(0x1C, 0x1E, 0x33),
            Place::Cafe => rgb(0xB8, 0x98, 0x74),
            Place::Dream => rgb(0xE0, 0xBE, 0xE8),
        }
    }
}

fn hash(mut v: u32) -> u32 {
    v ^= v >> 16;
    v = v.wrapping_mul(0x7feb_352d);
    v ^= v >> 15;
    v = v.wrapping_mul(0x846c_a68b);
    v ^ (v >> 16)
}

fn to_i32(v: u32) -> i32 {
    i32::try_from(v).unwrap_or(i32::MAX)
}

fn to_u32(v: i32) -> u32 {
    u32::try_from(v).unwrap_or(0)
}

/// A filled rectangle, skipped when it misses the band.
fn rect<D>(t: &mut D, x: i32, y: i32, w: i32, h: i32, c: Rgb565) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    let clip = t.bounding_box();
    let top = clip.top_left.y;
    let bottom = top + to_i32(clip.size.height);
    if y + h <= top || y >= bottom || w <= 0 || h <= 0 {
        return Ok(());
    }
    t.fill_solid(
        &Rectangle::new(Point::new(x, y), Size::new(to_u32(w), to_u32(h))),
        c,
    )
}

fn disc<D>(t: &mut D, cx: i32, cy: i32, r: i32, c: Rgb565) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    let clip = t.bounding_box();
    let top = clip.top_left.y;
    let bottom = top + to_i32(clip.size.height);
    if cy + r < top || cy - r >= bottom {
        return Ok(());
    }
    Circle::new(Point::new(cx - r, cy - r), to_u32(2 * r + 1))
        .into_styled(PrimitiveStyle::with_fill(c))
        .draw(t)
}

/// Horizontal bands from `top` to `bottom`, stepping from color `a` to `b`.
fn gradient<D>(t: &mut D, top: i32, bottom: i32, a: [u8; 3], b: [u8; 3]) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    const STEPS: i32 = 8;
    let band = (bottom - top + STEPS - 1) / STEPS;
    for i in 0..STEPS {
        let mix = |x: u8, y: u8| {
            let (x, y) = (i32::from(x), i32::from(y));
            u8::try_from(x + (y - x) * i / (STEPS - 1)).unwrap_or(0)
        };
        let c = rgb(mix(a[0], b[0]), mix(a[1], b[1]), mix(a[2], b[2]));
        let y = top + i * band;
        rect(t, 0, y, W, band.min(bottom - y), c)?;
    }
    Ok(())
}

/// Draws `place` at `t_ms`. Does nothing for [`Place::Pink`], which the app
/// draws itself.
pub fn draw<D>(t: &mut D, place: Place, t_ms: u32) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    match place {
        Place::Pink => Ok(()),
        Place::Club => club(t),
        Place::Beach => beach(t, t_ms),
        Place::Fall => fall(t, t_ms),
        Place::Cinema => cinema(t, t_ms),
        Place::City => city(t, t_ms),
        Place::Museum => museum(t),
        Place::Night => night(t, t_ms),
        Place::Cafe => cafe(t),
        Place::Dream => dream(t, t_ms),
    }
}

/// A wooden floor from `FLOOR_Y` down: planks with staggered seams.
fn planks<D>(t: &mut D, light: Rgb565, line: Rgb565) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    rect(t, 0, FLOOR_Y, W, H - FLOOR_Y, light)?;
    let mut y = FLOOR_Y;
    let mut row = 0;
    while y < H {
        rect(t, 0, y, W, 2, line)?;
        let offset = if row % 2 == 0 { 0 } else { 40 };
        let mut x = offset;
        while x < W {
            rect(t, x, y, 2, 16, line)?;
            x += 80;
        }
        y += 16;
        row += 1;
    }
    Ok(())
}

fn club<D>(t: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    rect(t, 0, 0, W, FLOOR_Y, rgb(0xFC, 0xE6, 0xDA))?;
    // A soft stripe of wallpaper and a skirting board.
    let mut x = 0;
    while x < W {
        rect(t, x, 0, 12, FLOOR_Y - 12, rgb(0xFA, 0xDD, 0xCF))?;
        x += 32;
    }
    rect(t, 0, FLOOR_Y - 12, W, 12, rgb(0xC9, 0x8B, 0x5E))?;
    planks(t, rgb(0xE6, 0xBE, 0x91), rgb(0xCF, 0xA2, 0x73))?;
    // The door she came through, a little open.
    rect(t, 186, 56, 64, FLOOR_Y - 56, rgb(0x6E, 0x45, 0x2C))?;
    rect(t, 192, 62, 52, FLOOR_Y - 62, rgb(0xFF, 0xF6, 0xE0))?;
    rect(t, 200, 62, 44, FLOOR_Y - 62, rgb(0xB9, 0x7A, 0x4E))?;
    rect(t, 206, 70, 32, 30, rgb(0xA4, 0x69, 0x41))?;
    rect(t, 206, 108, 32, 30, rgb(0xA4, 0x69, 0x41))?;
    disc(t, 236, 104, 2, rgb(0xFF, 0xD9, 0x5A))?;
    // A music poster, on the left wall below the title.
    rect(t, 10, 82, 40, 52, rgb(0x3B, 0x3F, 0x4A))?;
    rect(t, 14, 86, 32, 44, rgb(0xFF, 0xB6, 0xC8))?;
    Stamp::at(Point::new(16, 94), 3).art(t, &art::PROP_NOTE, 0, 0)
}

fn beach<D>(t: &mut D, t_ms: u32) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    gradient(t, 0, 112, [0x9E, 0xD8, 0xF0], [0xE3, 0xF6, 0xFF])?;
    rect(t, 0, 112, W, FLOOR_Y - 112, rgb(0x5E, 0xB8, 0xE0))?;
    rect(t, 0, 112, W, 4, rgb(0x46, 0x9F, 0xCC))?;
    // Waves roll along the sea, three rows at their own speeds.
    for (row, speed) in [(0, 30u32), (1, 45), (2, 60)] {
        let y = 120 + row * 10;
        let shift = to_i32(t_ms / speed % 64);
        let mut x = -64 + shift + row * 21;
        while x < W {
            rect(t, x, y, 18, 2, rgb(0xEA, 0xF8, 0xFF))?;
            x += 64;
        }
    }
    // Foam where the sea meets the sand, coming and going.
    let foam = to_i32(t_ms / 120 % 8);
    rect(t, 0, FLOOR_Y - 4 + foam / 4, W, 4, rgb(0xF4, 0xFB, 0xFF))?;
    rect(t, 0, FLOOR_Y, W, H - FLOOR_Y, rgb(0xF6, 0xDF, 0xA6))?;
    for k in 0..60u32 {
        let h = hash(k ^ 0xbeac);
        let x = to_i32(h % 480);
        let y = FLOOR_Y + 6 + to_i32(h / 480 % 118);
        rect(t, x, y, 2, 2, rgb(0xE3, 0xC5, 0x84))?;
    }
    // A shell or two.
    disc(t, 60, 238, 4, rgb(0xFF, 0xC8, 0xD4))?;
    disc(t, 232, 252, 3, rgb(0xFF, 0xE9, 0xF0))
}

fn tree<D>(t: &mut D, x: i32, base: i32, size: i32, colors: [Rgb565; 3]) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    rect(
        t,
        x - size / 6,
        base - size,
        size / 3,
        size,
        rgb(0x8B, 0x5A, 0x3C),
    )?;
    disc(t, x, base - size - size / 3, size * 2 / 3, colors[0])?;
    disc(t, x - size / 2, base - size, size / 2, colors[1])?;
    disc(t, x + size / 2, base - size - size / 6, size / 2, colors[2])
}

fn fall<D>(t: &mut D, t_ms: u32) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    gradient(t, 0, FLOOR_Y, [0xFF, 0xDE, 0xC2], [0xFF, 0xF2, 0xE3])?;
    let (orange, red, gold) = (
        rgb(0xF5, 0x9E, 0x4C),
        rgb(0xE0, 0x6A, 0x4A),
        rgb(0xF2, 0xC1, 0x4E),
    );
    tree(t, 40, FLOOR_Y + 4, 66, [orange, red, gold])?;
    tree(t, 210, FLOOR_Y + 4, 54, [gold, orange, red])?;
    tree(t, 470, FLOOR_Y + 4, 60, [red, gold, orange])?;
    rect(t, 0, FLOOR_Y, W, H - FLOOR_Y, rgb(0xD9, 0xB0, 0x78))?;
    for k in 0..40u32 {
        let h = hash(k ^ 0xfa11);
        rect(
            t,
            to_i32(h % 480),
            FLOOR_Y + 4 + to_i32(h / 480 % 118),
            4,
            2,
            [orange, red, gold][(h % 3) as usize],
        )?;
    }
    // Leaves drift down, swaying as they go.
    let leaf = Stamp::at(Point::new(0, 0), 3);
    for k in 0..7u32 {
        let h = hash(k.wrapping_mul(0x9e37) ^ 0x1eaf);
        let period = 5_000 + h % 4_000;
        let p = to_i32((t_ms.wrapping_add(h)) % period);
        let y = -20 + p * (H + 40) / to_i32(period);
        let sway = (p / 300 % 8 - 4).abs() * 4;
        let x = to_i32(h % 440) + sway + p / 60;
        Stamp {
            origin: Point::new(x % W, y),
            ..leaf
        }
        .art(t, &art::PROP_LEAF, 0, 0)?;
    }
    Ok(())
}

fn cinema<D>(t: &mut D, t_ms: u32) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    rect(t, 0, 0, W, H, rgb(0x2B, 0x21, 0x40))?;
    // The screen, glowing and flickering a little.
    let glow = if (t_ms / 140).is_multiple_of(7) {
        0xD8
    } else {
        0xE8
    };
    rect(t, 26, 18, 428, 106, rgb(0x44, 0x38, 0x66))?;
    rect(t, 32, 24, 416, 94, rgb(glow, 0xEE, 0xFF))?;
    // Curtains.
    for x in [0, 446] {
        rect(t, x, 0, 34, FLOOR_Y, rgb(0x8E, 0x2B, 0x3A))?;
        let mut fold = x + 6;
        while fold < x + 34 {
            rect(t, fold, 0, 3, FLOOR_Y, rgb(0x6E, 0x1F, 0x2C))?;
            fold += 10;
        }
    }
    rect(t, 0, FLOOR_Y, W, H - FLOOR_Y, rgb(0x3A, 0x2E, 0x52))?;
    // Rows of seats near the front.
    for (y, step) in [(220, 0), (244, 22)] {
        let mut x = -10 + step;
        while x < W {
            rect(t, x, y, 36, 20, rgb(0x9C, 0x2E, 0x45))?;
            rect(t, x + 4, y + 4, 28, 8, rgb(0xB8, 0x3E, 0x58))?;
            x += 44;
        }
    }
    Ok(())
}

fn city<D>(t: &mut D, t_ms: u32) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    gradient(t, 0, FLOOR_Y, [0xFF, 0xB8, 0xC8], [0xFF, 0xDD, 0xB8])?;
    // A row of buildings, each with windows that light up now and then.
    let mut x = 0;
    let mut k = 0u32;
    while x < W {
        let h = hash(k ^ 0xc17);
        let w = 40 + to_i32(h % 30);
        let tall = 60 + to_i32(h / 30 % 70);
        let top = FLOOR_Y - tall;
        let body = if k.is_multiple_of(2) {
            rgb(0x6B, 0x5B, 0x8A)
        } else {
            rgb(0x7D, 0x6C, 0xA0)
        };
        rect(t, x, top, w, tall, body)?;
        let mut wy = top + 8;
        let mut n = 0u32;
        while wy < FLOOR_Y - 10 {
            let mut wx = x + 6;
            while wx < x + w - 8 {
                let lit = !hash(k * 97 + n + t_ms / 2_000).is_multiple_of(3);
                let c = if lit {
                    rgb(0xFF, 0xE3, 0x8A)
                } else {
                    rgb(0x55, 0x48, 0x70)
                };
                rect(t, wx, wy, 6, 8, c)?;
                wx += 12;
                n += 1;
            }
            wy += 14;
        }
        x += w + 4;
        k += 1;
    }
    rect(t, 0, FLOOR_Y, W, H - FLOOR_Y, rgb(0xCF, 0xC8, 0xDC))?;
    rect(t, 0, FLOOR_Y, W, 4, rgb(0xA8, 0xA0, 0xBC))?;
    let mut sx = 0;
    while sx < W {
        rect(t, sx, FLOOR_Y + 4, 2, H - FLOOR_Y, rgb(0xBF, 0xB8, 0xCE))?;
        sx += 48;
    }
    Ok(())
}

fn museum<D>(t: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    rect(t, 0, 0, W, FLOOR_Y, rgb(0xF7, 0xF4, 0xEF))?;
    rect(t, 0, FLOOR_Y - 8, W, 8, rgb(0xDD, 0xD5, 0xC8))?;
    // Paintings in gold frames, each under a spotlight.
    let paintings: [(i32, i32, i32, [[u8; 3]; 3]); 4] = [
        (
            14,
            52,
            64,
            [[0x9E, 0xD8, 0xF0], [0x5E, 0xB8, 0xE0], [0xF6, 0xDF, 0xA6]],
        ),
        (
            96,
            40,
            72,
            [[0xFF, 0xB8, 0xC8], [0xE0, 0x6A, 0x4A], [0xF2, 0xC1, 0x4E]],
        ),
        (
            186,
            52,
            56,
            [[0x9C, 0xC8, 0x6A], [0x5E, 0x8C, 0x3A], [0xFF, 0xF3, 0xD6]],
        ),
        (
            300,
            36,
            80,
            [[0xB4, 0x8A, 0xE0], [0xFF, 0xD9, 0x5A], [0x6B, 0x5B, 0x8A]],
        ),
    ];
    for (x, y, w, [a, b, c]) in paintings {
        let h = w * 3 / 4;
        Triangle::new(
            Point::new(x + w / 2, 0),
            Point::new(x - 8, y + h + 20),
            Point::new(x + w + 8, y + h + 20),
        )
        .into_styled(PrimitiveStyle::with_fill(rgb(0xFF, 0xFB, 0xF0)))
        .draw(t)?;
        rect(t, x - 5, y - 5, w + 10, h + 10, rgb(0xC9, 0x9A, 0x3A))?;
        rect(t, x, y, w, h, rgb(a[0], a[1], a[2]))?;
        rect(t, x, y + h / 2, w, h / 2, rgb(b[0], b[1], b[2]))?;
        disc(t, x + w / 3, y + h / 3, w / 7, rgb(c[0], c[1], c[2]))?;
    }
    rect(t, 0, FLOOR_Y, W, H - FLOOR_Y, rgb(0xE9, 0xD7, 0xBF))?;
    rect(t, 0, FLOOR_Y + 30, W, 6, rgb(0xF2, 0xE5, 0xD3))?;
    Ok(())
}

fn night<D>(t: &mut D, t_ms: u32) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    gradient(t, 0, FLOOR_Y, [0x1E, 0x23, 0x47], [0x3A, 0x40, 0x75])?;
    // Stars, twinkling.
    for k in 0..36u32 {
        let h = hash(k ^ 0x57a5);
        let x = to_i32(h % 480);
        let y = to_i32(h / 480 % 130);
        let bright = !(t_ms / 400 + h).is_multiple_of(5);
        let c = if bright {
            rgb(0xFF, 0xF6, 0xD6)
        } else {
            rgb(0x8A, 0x90, 0xC0)
        };
        rect(t, x, y, 2, 2, c)?;
    }
    disc(t, 300, 40, 18, rgb(0xFF, 0xF3, 0xD6))?;
    disc(t, 307, 34, 16, rgb(0x2A, 0x30, 0x5E))?;
    // A street lamp and its glow.
    disc(t, 230, 64, 30, rgb(0x4A, 0x4A, 0x70))?;
    disc(t, 230, 64, 16, rgb(0x8E, 0x82, 0x86))?;
    rect(t, 228, 64, 4, FLOOR_Y - 64, rgb(0x16, 0x18, 0x30))?;
    disc(t, 230, 62, 6, rgb(0xFF, 0xE3, 0x8A))?;
    rect(t, 0, FLOOR_Y, W, H - FLOOR_Y, rgb(0x2C, 0x2F, 0x4A))?;
    rect(t, 0, FLOOR_Y, W, 3, rgb(0x45, 0x48, 0x6E))?;
    Ok(())
}

fn cafe<D>(t: &mut D) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    rect(t, 0, 0, W, FLOOR_Y, rgb(0xF1, 0xDE, 0xC6))?;
    rect(t, 0, 96, W, FLOOR_Y - 96, rgb(0xB0, 0x7B, 0x52))?;
    rect(t, 0, 96, W, 4, rgb(0x8B, 0x5A, 0x3C))?;
    // A window onto the day.
    rect(t, 20, 18, 120, 70, rgb(0x8B, 0x5A, 0x3C))?;
    rect(t, 26, 24, 108, 58, rgb(0xBD, 0xE7, 0xFA))?;
    rect(t, 78, 24, 4, 58, rgb(0x8B, 0x5A, 0x3C))?;
    disc(t, 52, 70, 10, rgb(0x9C, 0xC8, 0x6A))?;
    // Shelves of cups.
    for y in [34, 66] {
        rect(t, 168, y + 16, 100, 4, rgb(0x8B, 0x5A, 0x3C))?;
        for (i, c) in [
            rgb(0xFF, 0xB6, 0xC8),
            rgb(0x9C, 0xC8, 0x6A),
            rgb(0xFF, 0xF3, 0xD6),
            rgb(0x7F, 0xB3, 0xFF),
            rgb(0xF5, 0x9E, 0x4C),
        ]
        .into_iter()
        .enumerate()
        {
            let x = 174 + to_i32(u32::try_from(i).unwrap_or(0)) * 19;
            rect(t, x, y + 4, 12, 12, c)?;
        }
    }
    // A hanging lamp.
    rect(t, 300, 0, 2, 26, rgb(0x3B, 0x3F, 0x4A))?;
    Triangle::new(
        Point::new(301, 22),
        Point::new(286, 40),
        Point::new(316, 40),
    )
    .into_styled(PrimitiveStyle::with_fill(rgb(0x5E, 0x8C, 0x3A)))
    .draw(t)?;
    disc(t, 301, 42, 4, rgb(0xFF, 0xE3, 0x8A))?;
    // A checked floor.
    rect(t, 0, FLOOR_Y, W, H - FLOOR_Y, rgb(0xE2, 0xC9, 0xA8))?;
    let mut y = FLOOR_Y;
    let mut row = 0;
    while y < H {
        let mut x = if row % 2 == 0 { 0 } else { 20 };
        while x < W {
            rect(t, x, y, 20, 20, rgb(0xCF, 0xB0, 0x8A))?;
            x += 40;
        }
        y += 20;
        row += 1;
    }
    Ok(())
}

fn dream<D>(t: &mut D, t_ms: u32) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    gradient(t, 0, H, [0xFF, 0xC8, 0xDD], [0xD6, 0xC4, 0xFF])?;
    // White stars twinkling: each grows, shines, shrinks and rests on its
    // own beat.
    for k in 0..22u32 {
        let h = hash(k ^ 0x5a2c);
        let at = Point::new(to_i32(h % 468), to_i32(h / 468 % 258));
        let beat = 1_600 + h % 1_400;
        match (t_ms.wrapping_add(h)) % beat * 4 / beat {
            0 | 2 => Stamp::at(at + Point::new(1, 1), 1).art(t, &art::PROP_STAR_SMALL, 0, 0)?,
            1 => Stamp::at(at, 1).art(t, &art::PROP_STAR, 0, 0)?,
            _ => rect(t, at.x + 3, at.y + 3, 1, 1, rgb(0xFF, 0xFF, 0xFF))?,
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::vec;
    use std::vec::Vec;

    use super::*;
    use crate::frame::{Band, HEIGHT, WIDTH};

    fn frame(place: Place, t: u32, rows: usize) -> Vec<u16> {
        let mut out = vec![0u16; WIDTH * HEIGHT];
        let mut band = vec![0u16; WIDTH * rows];
        for top in (0..HEIGHT).step_by(rows) {
            band.fill(0);
            draw(&mut Band::new(&mut band, top), place, t).unwrap();
            let n = band.len().min(out.len() - top * WIDTH);
            out[top * WIDTH..top * WIDTH + n].copy_from_slice(&band[..n]);
        }
        out
    }

    #[test]
    fn every_place_has_a_name_that_finds_it() {
        for (p, n) in Place::ALL.iter().zip(Place::NAMES) {
            assert_eq!(Place::from_name(n), Some(*p));
        }
        assert_eq!(Place::from_name("moon"), None);
    }

    #[test]
    fn every_place_but_pink_covers_the_whole_screen() {
        for place in Place::ALL.into_iter().filter(|p| *p != Place::Pink) {
            let f = frame(place, 1_000, 16);
            let empty = f.iter().filter(|&&p| p == 0).count();
            assert_eq!(empty, 0, "{place:?} leaves {empty} pixels undrawn");
        }
        assert!(frame(Place::Pink, 0, 16).iter().all(|&p| p == 0));
    }

    #[test]
    fn drawing_in_bands_matches_drawing_whole() {
        for place in Place::ALL {
            assert!(
                frame(place, 2_345, 16) == frame(place, 2_345, 272),
                "{place:?}"
            );
        }
    }

    #[test]
    fn the_lively_places_move() {
        for place in [
            Place::Beach,
            Place::Fall,
            Place::Night,
            Place::City,
            Place::Dream,
        ] {
            assert!(
                frame(place, 0, 16) != frame(place, 3_000, 16),
                "{place:?} stands still"
            );
        }
    }
}
