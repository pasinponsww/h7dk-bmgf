//! ASCII DOG
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::{DrawTarget, Point, Size};
use embedded_graphics::primitives::Rectangle;

use crate::theme;

/// Width of the drawing area in sprite pixels.
pub const W: usize = 34;
/// Height of the drawing area in sprite pixels, with room above for jumps and hearts.
pub const H: usize = 40;
/// Number of cells in the drawing area.
pub const CELLS: usize = W * H;
/// Character of a transparent cell.
pub const CLEAR: u8 = b'.';

const MARGIN_X: i32 = 1;
const HEADROOM: u8 = 4;
const SPRITE_W: usize = 32;
const SPRITE_H: usize = 36;

const BASE: [&str; SPRITE_H] = [
    "KK............................KK",
    "KPQK........................KPQK",
    "KPPPQK....................KPPPQK",
    "KQPPPQK..................KPPPPQK",
    ".KPPPPSK................KWPPPQK.",
    ".KPPPPSSK..............KWWPPPQK.",
    ".KQPPPWSSK............KWWWPPPQK.",
    "..KPPPWWSSKKKKKKKKKKKKWWWWPPQK..",
    "..KQPWWWWWWWWWWWWWWWWWWWWWWPQK..",
    "...KPWWWWWWWWWWWWWWWWWWWWWSQK...",
    "...KWWWWWWWWWWWWWWWWWWWWWWSSK...",
    "...KWWWWWWWWWWWWWWWWWWWWWWSSK...",
    "..KWWWWWWWWWWWWWWWWWWWWWWWWSSK..",
    "..KWWWWWWWWWWWWWWWWWWWWWWWWSSK..",
    "..KWWWWWWWWWWWWWWWWWWWWWWWWSSK..",
    "..KWWWWWWWWWWWWWWWWWWWWWWWWSSK..",
    "..KSWWWWWWWWWWWWWWWWWWWWWWWSSK..",
    "...KWWWWWWWWWWWWWWWWWWWWWWSSK...",
    "...KSWWWWWWWWWWWWWWWWWWWWWSSK...",
    "....KSWWWWWWWWWWWWWWWWWWWSSK....",
    ".....KSSWWWWWWWWWWWWWWWWSSK.....",
    "......KKSWWWWWWWWWWWWWSSKK......",
    "........KSSSSSSSSSSSSSSK........",
    "........KHHHHHHHHHHHHHHK........",
    ".......KWWWWWWDDDDWWWWSSK.......",
    "......KWWWWWWWWDDWWWWWWSSK......",
    ".....KWWWSWWWWWWWWWWWWSWSSK.....",
    "....KWWSSKWWWWWWWWWWSSKWWSSK....",
    "....KWWSSKWWWWSWWSWWSSKWWSSK....",
    "...KWWWSSKWWSSK..KWWSSKWWWSSK...",
    "...KWWWSSKWWSSK..KWWSSKWWWSSK...",
    "...KWWWSSKWWSSK..KWWSSKWWWSSK...",
    "...KWWWSSKWWSSK..KWWSSKWWWSSK...",
    "..KWWWWSSKWWWSSKKWWWSSKWWWWSSK..",
    ".KSSSSSSSKSSSSSKKSSSSSKSSSSSSSK.",
    ".KKKKKKKKKKKKKKKKKKKKKKKKKKKKKK.",
];

const _: () = assert!(BASE[0].len() == SPRITE_W, "the art is 32 pixels wide");

const EYE_LEFT: (i32, i32) = (6, 11);
const EYE_RIGHT: (i32, i32) = (21, 11);
const NOSE: (i32, i32) = (14, 17);
const MOUTH_AT: (i32, i32) = (12, 19);
const TAIL_AT: (i32, i32) = (25, 25);
const CHEEKS: [(i32, i32); 8] = [
    (4, 16),
    (5, 16),
    (6, 16),
    (5, 17),
    (25, 16),
    (26, 16),
    (27, 16),
    (26, 17),
];
const NOSE_ART: [&str; 3] = ["NWNN", "NNNN", ".NN."];
const HEART: [&str; 4] = [".H.H.", "HHHHH", ".HHH.", "..H.."];
const HEART_X: i32 = 14;

/// What the eyes look like.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Eyes {
    /// Round and open, with a glint.
    Open,
    /// A thin line.
    Blink,
    /// Happy arches.
    Happy,
    /// Big and surprised.
    Wide,
    /// Hearts.
    Love,
    /// Half closed, looking down.
    Down,
    /// Rings with a tiny pupil.
    Scared,
}

/// What the mouth looks like.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Mouth {
    /// The classic dog smile.
    Smile,
    /// Open with the tongue out.
    Tongue,
    /// A round "o".
    Open,
    /// A short flat line.
    Flat,
    /// Wobbly.
    Wavy,
}

const fn eyes_art(e: Eyes) -> [&'static str; 5] {
    match e {
        Eyes::Open => [".EEE.", "EWEEE", "EEEEE", "EEEEE", ".EEE."],
        Eyes::Blink => [".....", ".....", "EEEEE", ".....", "....."],
        Eyes::Happy => [".....", ".EEE.", "E...E", ".....", "....."],
        Eyes::Wide => ["EEEEE", "EWEEE", "EEEEE", "EEEEE", "EEEEE"],
        Eyes::Love => [".H.H.", "HHHHH", "HHHHH", ".HHH.", "..H.."],
        Eyes::Down => [".....", ".....", ".EEE.", "EEEEE", ".EEE."],
        Eyes::Scared => ["EEEEE", "E...E", "E.E.E", "E...E", "EEEEE"],
    }
}

const fn mouth_art(m: Mouth) -> [&'static str; 5] {
    match m {
        Mouth::Smile => ["...KK...", "K..KK..K", ".KK..KK.", "........", "........"],
        Mouth::Tongue => ["...KK...", "K..KK..K", ".KKTTKK.", "...TT...", "...TT..."],
        Mouth::Open => ["...KK...", "..K..K..", "..K..K..", "...KK...", "........"],
        Mouth::Flat => ["...KK...", "..KKKK..", "........", "........", "........"],
        Mouth::Wavy => ["...KK...", ".K.KK.K.", "K.K..K.K", "........", "........"],
    }
}

const fn tail_art(frame: u8) -> [&'static str; 9] {
    match frame {
        0 => [
            ".....KK", "....KWK", "....KWK", "...KWWK", "...KWK.", "..KWWK.", "..KWK..", ".KWK...",
            ".KK....",
        ],
        1 => [
            ".......", ".......", ".....KK", "....KWK", "...KWWK", "...KWK.", "..KWWK.", ".KWK...",
            ".KK....",
        ],
        _ => [
            ".......", ".......", ".......", ".......", ".......", "....KKK", "..KKWWK", ".KWWWK.",
            ".KKKK..",
        ],
    }
}

/// How the dog looks at one moment.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Pose {
    /// The eyes.
    pub eyes: Eyes,
    /// The mouth.
    pub mouth: Mouth,
    /// Tail frame 0 to 2, or `None` when the tail is tucked away.
    pub tail: Option<u8>,
    /// How many sprite pixels the dog is lifted off the ground (0 to 4).
    pub lift: u8,
    /// Sideways shake in sprite pixels (-1, 0 or 1).
    pub dx: i8,
    /// Pink cheeks.
    pub blush: bool,
    /// Row of a floating heart above the head, or `None`.
    pub heart: Option<u8>,
}

/// How the dog feels. A story chapter picks one.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Mood {
    /// Sits, blinks, wags slowly.
    Idle,
    /// Tongue out, wags fast, small bounce.
    Happy,
    /// Wide eyes, stiff tail.
    Wow,
    /// Blushes and looks down.
    Shy,
    /// Heart eyes and a floating heart.
    Love,
    /// Shivers with the tail tucked.
    Scared,
    /// Jumps around.
    Cheer,
}

impl Mood {
    /// Every mood, in a fixed order that the simulator also uses.
    pub const ALL: [Mood; 7] = [
        Mood::Idle,
        Mood::Happy,
        Mood::Wow,
        Mood::Shy,
        Mood::Love,
        Mood::Scared,
        Mood::Cheer,
    ];
    /// The names a story file may use, in the same order as `ALL`.
    pub const NAMES: [&'static str; 7] = ["idle", "happy", "wow", "shy", "love", "scared", "cheer"];

    /// The mood called `name`, if there is one.
    pub fn from_name(name: &str) -> Option<Mood> {
        Self::NAMES
            .iter()
            .position(|n| *n == name)
            .map(|i| Self::ALL[i])
    }

    /// The name used in story files.
    pub fn name(self) -> &'static str {
        Self::NAMES[self.index()]
    }

    /// Position in `ALL`.
    pub fn index(self) -> usize {
        Self::ALL.iter().position(|m| *m == self).unwrap_or(0)
    }

    /// The mood at position `i` in `ALL`, or `Idle` if there is none.
    pub fn from_index(i: usize) -> Mood {
        Self::ALL.get(i).copied().unwrap_or(Mood::Idle)
    }
}

fn blinking(t_ms: u32) -> bool {
    t_ms % 3_200 < 140
}

fn wag(t_ms: u32, step_ms: u32) -> Option<u8> {
    Some([0, 1, 2, 1][(t_ms / step_ms % 4) as usize])
}

fn bounce(t_ms: u32, period_ms: u32, peak: u8) -> u8 {
    let phase = t_ms % period_ms;
    let half = period_ms / 2;
    let up = if phase < half {
        phase
    } else {
        period_ms - phase
    };
    u8::try_from(up * u32::from(peak) / half).unwrap_or(peak)
}

/// How the dog looks in `mood` at `t_ms` milliseconds.
pub fn pose(mood: Mood, t_ms: u32) -> Pose {
    let still = Pose {
        eyes: Eyes::Open,
        mouth: Mouth::Smile,
        tail: wag(t_ms, 350),
        lift: 0,
        dx: 0,
        blush: false,
        heart: None,
    };
    match mood {
        Mood::Idle => Pose {
            eyes: if blinking(t_ms) {
                Eyes::Blink
            } else {
                Eyes::Open
            },
            ..still
        },
        Mood::Happy => Pose {
            eyes: if t_ms / 900 % 3 == 2 {
                Eyes::Happy
            } else {
                Eyes::Open
            },
            mouth: Mouth::Tongue,
            tail: wag(t_ms, 160),
            lift: bounce(t_ms, 700, 1),
            ..still
        },
        Mood::Wow => Pose {
            eyes: Eyes::Wide,
            mouth: Mouth::Open,
            tail: Some(0),
            lift: bounce(t_ms, 1_600, 2),
            ..still
        },
        Mood::Shy => Pose {
            eyes: Eyes::Down,
            mouth: Mouth::Flat,
            tail: wag(t_ms, 600),
            blush: true,
            ..still
        },
        Mood::Love => Pose {
            eyes: Eyes::Love,
            mouth: Mouth::Tongue,
            tail: wag(t_ms, 180),
            lift: bounce(t_ms, 900, 1),
            blush: true,
            heart: Some(u8::try_from(7 - t_ms % 1_400 * 7 / 1_400).unwrap_or(0)),
            ..still
        },
        Mood::Scared => Pose {
            eyes: Eyes::Scared,
            mouth: Mouth::Wavy,
            tail: None,
            dx: if (t_ms / 60).is_multiple_of(2) { -1 } else { 1 },
            ..still
        },
        Mood::Cheer => Pose {
            eyes: Eyes::Happy,
            mouth: Mouth::Tongue,
            tail: wag(t_ms, 120),
            lift: bounce(t_ms, 520, HEADROOM),
            ..still
        },
    }
}

fn pick(art: &[&str], at: (i32, i32), x: i32, y: i32) -> u8 {
    let (Ok(col), Ok(row)) = (usize::try_from(x - at.0), usize::try_from(y - at.1)) else {
        return CLEAR;
    };
    art.get(row)
        .and_then(|r| r.as_bytes().get(col))
        .copied()
        .unwrap_or(CLEAR)
}

fn coord(v: usize) -> i32 {
    i32::try_from(v).unwrap_or(i32::MAX)
}

/// The character at column `x`, row `y` of the drawing area for `pose`.
/// `CLEAR` means nothing is drawn there.
pub fn cell(pose: &Pose, x: usize, y: usize) -> u8 {
    let (x, y) = (coord(x), coord(y));
    if let Some(row) = pose.heart {
        let c = pick(&HEART, (HEART_X, i32::from(row)), x, y);
        if c != CLEAR {
            return c;
        }
    }
    let sx = x - MARGIN_X - i32::from(pose.dx);
    let sy = y - i32::from(HEADROOM - pose.lift.min(HEADROOM));
    let mut c = pick(&BASE, (0, 0), sx, sy);
    if let Some(frame) = pose.tail {
        let t = pick(&tail_art(frame), TAIL_AT, sx, sy);
        if t != CLEAR {
            c = t;
        }
    }
    if pose.blush && CHEEKS.contains(&(sx, sy)) {
        c = b'B';
    }
    for (art, at) in [
        (&NOSE_ART[..], NOSE),
        (&mouth_art(pose.mouth)[..], MOUTH_AT),
        (&eyes_art(pose.eyes)[..], EYE_LEFT),
        (&eyes_art(pose.eyes)[..], EYE_RIGHT),
    ] {
        let o = pick(art, at, sx, sy);
        if o != CLEAR {
            c = o;
        }
    }
    c
}

/// Fills `out` with the characters of the whole drawing area, row by row.
pub fn render(pose: &Pose, out: &mut [u8; CELLS]) {
    for (i, slot) in out.iter_mut().enumerate() {
        *slot = cell(pose, i % W, i / W);
    }
}

/// Paints the dog with its top-left corner at `origin`, each sprite pixel
/// drawn as a `scale` by `scale` square.
pub fn draw<D>(target: &mut D, origin: Point, scale: u32, pose: &Pose) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    let px = i32::try_from(scale).unwrap_or(1);
    for y in 0..H {
        let mut x = 0;
        while x < W {
            let c = cell(pose, x, y);
            let start = x;
            while x < W && cell(pose, x, y) == c {
                x += 1;
            }
            if let Some(color) = theme::mascot_color(c) {
                let top_left = origin + Point::new(coord(start) * px, coord(y) * px);
                let size = Size::new(u32::try_from(x - start).unwrap_or(0) * scale, scale);
                target.fill_solid(&Rectangle::new(top_left, size), color)?;
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use core::convert::Infallible;
    use std::{vec, vec::Vec};

    use embedded_graphics::pixelcolor::Rgb565;
    use embedded_graphics::prelude::{DrawTarget, OriginDimensions, Pixel, Point, Size};

    use super::*;

    struct Screen {
        pixels: [Option<Rgb565>; CELLS * 4],
    }

    impl OriginDimensions for Screen {
        fn size(&self) -> Size {
            Size::new(W as u32 * 2, H as u32 * 2)
        }
    }

    impl DrawTarget for Screen {
        type Color = Rgb565;
        type Error = Infallible;
        fn draw_iter<I: IntoIterator<Item = Pixel<Rgb565>>>(
            &mut self,
            it: I,
        ) -> Result<(), Infallible> {
            for Pixel(p, c) in it {
                let (x, y) = (usize::try_from(p.x).unwrap(), usize::try_from(p.y).unwrap());
                assert!(
                    x < W * 2 && y < H * 2,
                    "pixel ({x}, {y}) is outside the drawing area"
                );
                self.pixels[y * W * 2 + x] = Some(c);
            }
            Ok(())
        }
    }

    fn frame(mood: Mood, t: u32) -> [u8; CELLS] {
        let mut out = [CLEAR; CELLS];
        render(&pose(mood, t), &mut out);
        out
    }

    #[test]
    fn the_base_art_is_a_full_grid_of_known_characters() {
        for row in BASE {
            assert_eq!(row.len(), SPRITE_W, "{row}");
            for c in row.bytes() {
                assert!(
                    c == CLEAR || theme::mascot_rgb(c).is_some(),
                    "{}",
                    char::from(c)
                );
            }
        }
    }

    #[test]
    fn the_outline_is_the_same_on_the_left_and_the_right() {
        // The shading is not mirrored (the light comes from one side), the shape is.
        for (i, row) in BASE.iter().enumerate() {
            let shape: Vec<bool> = row.bytes().map(|c| c != CLEAR).collect();
            let mirrored: Vec<bool> = shape.iter().rev().copied().collect();
            assert_eq!(shape, mirrored, "row {i}");
            let edges: Vec<bool> = row.bytes().map(|c| c == b'K').collect();
            let mirrored: Vec<bool> = edges.iter().rev().copied().collect();
            assert_eq!(edges, mirrored, "row {i} outline");
        }
    }

    #[test]
    fn the_dog_is_shaded_on_its_right_and_underneath() {
        let right: usize = BASE
            .iter()
            .map(|r| r.bytes().skip(SPRITE_W / 2).filter(|c| *c == b'S').count())
            .sum();
        let left: usize = BASE
            .iter()
            .map(|r| r.bytes().take(SPRITE_W / 2).filter(|c| *c == b'S').count())
            .sum();
        assert!(right > left, "right {right}, left {left}");
        assert!(BASE[SPRITE_H - 2].contains('S'));
        assert!(
            BASE[22].contains("KSSSS"),
            "the neck is in the head's shadow"
        );
    }

    #[test]
    fn every_face_and_tail_piece_has_the_right_size_and_colors() {
        let eyes = [
            Eyes::Open,
            Eyes::Blink,
            Eyes::Happy,
            Eyes::Wide,
            Eyes::Love,
            Eyes::Down,
            Eyes::Scared,
        ];
        let mouths = [
            Mouth::Smile,
            Mouth::Tongue,
            Mouth::Open,
            Mouth::Flat,
            Mouth::Wavy,
        ];
        let mut parts: Vec<(Vec<&str>, usize)> = vec![];
        parts.extend(eyes.map(|e| (eyes_art(e).to_vec(), 5)));
        parts.extend(mouths.map(|m| (mouth_art(m).to_vec(), 8)));
        parts.extend((0..3).map(|f| (tail_art(f).to_vec(), 7)));
        parts.push((HEART.to_vec(), 5));
        parts.push((NOSE_ART.to_vec(), 4));
        for (art, width) in parts {
            for row in art {
                assert_eq!(row.len(), width, "{row}");
                for c in row.bytes() {
                    assert!(
                        c == CLEAR || theme::mascot_rgb(c).is_some(),
                        "{}",
                        char::from(c)
                    );
                }
            }
        }
    }

    #[test]
    fn the_face_sits_inside_the_head() {
        let p = pose(Mood::Wow, 0);
        for (ox, oy) in [EYE_LEFT, EYE_RIGHT] {
            for dy in 0..5 {
                for dx in 0..5 {
                    let base = pick(&BASE, (0, 0), ox + dx, oy + dy);
                    assert!(base == b'W' || base == b'S', "eye at ({dx}, {dy})");
                }
            }
        }
        for dy in 0..3 {
            for dx in 0..4 {
                assert_eq!(pick(&BASE, (0, 0), NOSE.0 + dx, NOSE.1 + dy), b'W');
            }
        }
        for (x, y) in CHEEKS {
            let base = pick(&BASE, (0, 0), x, y);
            assert!(base == b'W' || base == b'S', "cheek at ({x}, {y})");
        }
        let mut out = [CLEAR; CELLS];
        render(&p, &mut out);
        assert!(out.contains(&b'E') && out.contains(&b'N'));
    }

    #[test]
    fn mood_names_round_trip() {
        for (i, m) in Mood::ALL.iter().enumerate() {
            assert_eq!(Mood::from_name(m.name()), Some(*m));
            assert_eq!(m.index(), i);
            assert_eq!(Mood::from_index(i), *m);
        }
        assert_eq!(Mood::from_name("grumpy"), None);
        assert_eq!(Mood::from_index(99), Mood::Idle);
    }

    #[test]
    fn the_dog_blinks_now_and_then_but_not_all_the_time() {
        assert_eq!(pose(Mood::Idle, 0).eyes, Eyes::Blink);
        assert_eq!(pose(Mood::Idle, 1_000).eyes, Eyes::Open);
        assert_eq!(pose(Mood::Idle, 3_200).eyes, Eyes::Blink);
    }

    #[test]
    fn every_mood_stays_inside_the_drawing_area() {
        for mood in Mood::ALL {
            for t in (0..8_000).step_by(7) {
                let p = pose(mood, t);
                assert!(p.lift <= HEADROOM, "{mood:?} lifts too high");
                assert!(p.dx.abs() <= 1, "{mood:?} shakes too far");
                assert!(p.heart.is_none_or(|h| usize::from(h) + HEART.len() <= H));
                let f = frame(mood, t);
                let top = f
                    .iter()
                    .position(|c| *c != CLEAR)
                    .expect("something is drawn");
                assert!(top / W <= H - SPRITE_H + usize::from(HEADROOM));
                for y in 0..H {
                    let row = &f[y * W..(y + 1) * W];
                    assert!(row
                        .iter()
                        .all(|c| *c == CLEAR || theme::mascot_rgb(*c).is_some()));
                }
                assert_eq!(f[(H - 1) * W], CLEAR, "bottom corner is empty");
            }
        }
    }

    #[test]
    fn every_mood_moves() {
        for mood in Mood::ALL {
            let first = frame(mood, 0);
            let moves = (0..3_000).step_by(20).any(|t| frame(mood, t) != first);
            assert!(moves, "{mood:?} never changes");
        }
    }

    #[test]
    fn the_tail_wags_through_all_three_frames() {
        let seen: Vec<u8> = (0..1_400)
            .step_by(50)
            .filter_map(|t| pose(Mood::Idle, t).tail)
            .collect();
        for f in 0..3 {
            assert!(seen.contains(&f), "frame {f}");
        }
        assert_eq!(
            pose(Mood::Scared, 0).tail,
            None,
            "a scared dog tucks its tail"
        );
    }

    #[test]
    fn cheer_jumps_higher_than_idle_and_love_floats_a_heart() {
        let top = |m| (0..2_000).map(|t| pose(m, t).lift).max().unwrap();
        assert_eq!(top(Mood::Idle), 0);
        assert_eq!(top(Mood::Cheer), HEADROOM);
        assert!((0..1_400)
            .step_by(50)
            .any(|t| pose(Mood::Love, t).heart.is_some()));
        assert!(pose(Mood::Idle, 0).heart.is_none());
    }

    #[test]
    fn the_dog_has_white_fur_a_dark_outline_and_a_collar() {
        let f = frame(Mood::Idle, 1_000);
        assert!(f.contains(&b'W') && f.contains(&b'K') && f.contains(&b'S'));
        assert!(f.contains(&b'H'), "the collar is rose");
        assert!(!f.contains(&b'B'), "no blush when calm");
        assert!(frame(Mood::Shy, 0).contains(&b'B'));
    }

    #[test]
    fn draw_paints_exactly_the_cells_of_the_frame_scaled_up() {
        let p = pose(Mood::Happy, 400);
        let mut screen = Screen {
            pixels: [None; CELLS * 4],
        };
        draw(&mut screen, Point::new(0, 0), 2, &p).unwrap();
        for y in 0..H * 2 {
            for x in 0..W * 2 {
                let want = theme::mascot_color(cell(&p, x / 2, y / 2));
                assert_eq!(screen.pixels[y * W * 2 + x], want, "({x}, {y})");
            }
        }
    }
}
