//! Drawing a [`Stage`]: the two dogs and their props, over a place.
//!
//! Each dog is put together from the pixel art in `ui::art`: the body for
//! the way it faces (mirrored when it faces left), then its eyes, mouth,
//! blush, her bow and anything it wears. The dog further away is drawn
//! first, so the nearer one passes in front.

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::{DrawTarget, Point, Primitive, Size};
use embedded_graphics::primitives::{Ellipse, PrimitiveStyle};
use embedded_graphics::Drawable;
use ui::art::{self, *};
use ui::sprite::{Art, Piece, Stamp};

use crate::moment::{wear, Eyes, Facing, Mouth, Prop, Pup, Shown, Stage};

/// Screen pixels per art pixel for the dogs.
pub const SCALE: i32 = 3;
const FRONT_W: i32 = 26;
const SIDE_W: i32 = 28;
const CANVAS_H: i32 = 30;

fn canvas_w(f: Facing) -> i32 {
    match f {
        Facing::Left | Facing::Right => SIDE_W,
        Facing::Front | Facing::Back => FRONT_W,
    }
}

/// The top-left corner of a dog's canvas on the screen.
fn origin(p: &Pup) -> Point {
    Point::new(
        p.x - canvas_w(p.facing) * SCALE / 2,
        p.y - CANVAS_H * SCALE - p.lift,
    )
}

fn stamp(p: &Pup) -> Stamp {
    Stamp {
        origin: origin(p),
        scale: SCALE.unsigned_abs(),
        flip: (p.facing == Facing::Left).then_some(SIDE_W),
    }
}

/// The front eyes for a dog: hers have lashes.
fn front_eyes(p: &Pup) -> &'static Piece {
    match (p.eyes, p.her) {
        (Eyes::Open, false) => &EYES_OPEN,
        (Eyes::Open, true) => &EYES_LASH,
        (Eyes::Happy, false) => &EYES_HAPPY,
        (Eyes::Happy, true) => &EYES_LASH_HAPPY,
        (Eyes::Closed | Eyes::Chill, _) => &EYES_CHILL,
        (Eyes::Wow, _) => &EYES_WOW,
        (Eyes::Heart, _) => &EYES_HEART,
        (Eyes::HeartBig, _) => &EYES_HEART_BIG,
    }
}

fn mouth(m: Mouth) -> &'static Piece {
    match m {
        Mouth::Smile => &MOUTH_SMILE,
        Mouth::Wow => &MOUTH_WOW,
        Mouth::Grin => &MOUTH_GRIN,
        Mouth::Laugh => &MOUTH_LAUGH,
    }
}

/// What a dog wears, front pieces, in drawing order (glasses, then hats).
fn worn(bits: u8) -> impl Iterator<Item = &'static Piece> {
    [
        (wear::GLASSES, &WEAR_GLASSES),
        (wear::SUNGLASSES, &WEAR_SUNGLASSES),
        (wear::GLASSES_3D, &WEAR_3D_GLASSES),
        (wear::BLACK_HAT, &WEAR_BLACK_HAT),
        (wear::STRAW_HAT, &WEAR_STRAW_HAT),
    ]
    .into_iter()
    .filter(move |(bit, _)| bits & bit != 0)
    .map(|(_, p)| p)
}

/// Draws one dog.
pub fn pup<D>(target: &mut D, p: &Pup) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    let st = stamp(p);
    match p.facing {
        Facing::Front => {
            st.art(
                target,
                if p.step { &DOG_FRONT_STEP } else { &DOG_FRONT },
                0,
                0,
            )?;
            st.piece(target, front_eyes(p))?;
            st.piece(target, mouth(p.mouth))?;
            if p.blush {
                st.piece(target, &MOUTH_BLUSH)?;
            }
            if p.her {
                st.piece(target, &BOW)?;
            }
            for w in worn(p.wear) {
                st.piece(target, w)?;
            }
        }
        Facing::Back => {
            st.art(
                target,
                if p.step { &DOG_BACK_STEP } else { &DOG_BACK },
                0,
                0,
            )?;
            if p.her {
                st.piece(target, &BOW)?;
            }
            // From behind only the hats show.
            let hats = wear::BLACK_HAT | wear::STRAW_HAT;
            for w in worn(p.wear & hats) {
                st.piece(target, w)?;
            }
        }
        Facing::Left | Facing::Right => {
            st.art(
                target,
                if p.step { &DOG_SIDE_STEP } else { &DOG_SIDE },
                0,
                0,
            )?;
            let eyes = match p.eyes {
                Eyes::Happy | Eyes::Chill => Some(&SIDE_EYES_HAPPY),
                Eyes::Closed => Some(&SIDE_EYES_CLOSED),
                Eyes::Heart | Eyes::HeartBig => Some(&SIDE_EYES_HEART),
                Eyes::Open | Eyes::Wow => None,
            };
            if let Some(e) = eyes {
                st.piece(target, e)?;
            }
            if p.her {
                st.piece(target, &SIDE_BOW)?;
            }
        }
    }
    Ok(())
}

/// A soft shadow on the floor under a dog, smaller when it is in the air.
fn shadow<D>(target: &mut D, p: &Pup, color: Rgb565) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    let w = (60 - p.lift / 2).max(30);
    let h = 12 - (p.lift / 8).min(6);
    let w = if matches!(p.facing, Facing::Left | Facing::Right) {
        w + 8
    } else {
        w
    };
    Ellipse::new(
        Point::new(p.x - w / 2 + 3, p.y - h / 2 - 2),
        Size::new(w.unsigned_abs(), h.unsigned_abs()),
    )
    .into_styled(PrimitiveStyle::with_fill(color))
    .draw(target)
}

fn prop_art(p: Prop) -> &'static Art {
    match p {
        Prop::Thought => &art::PROP_THOUGHT,
        Prop::Heart => &art::PROP_HEART,
        Prop::Laptop => &art::PROP_LAPTOP,
        Prop::Sun => &art::PROP_SUN,
        Prop::Popcorn => &art::PROP_POPCORN,
        Prop::Gelato => &art::PROP_GELATO,
        Prop::Boba => &art::PROP_BOBA,
        Prop::Sushi => &art::PROP_SUSHI,
        Prop::Ramen => &art::PROP_RAMEN,
        Prop::Phone => &art::PROP_PHONE,
        Prop::Matcha => &art::PROP_MATCHA,
        Prop::Note => &art::PROP_NOTE,
        Prop::Sparkle => &art::PROP_SPARKLE,
    }
}

/// Draws a prop.
pub fn prop<D>(target: &mut D, s: &Shown) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    Stamp::at(Point::new(s.x, s.y), s.scale).art(target, prop_art(s.prop), 0, 0)
}

/// Draws the dogs and props of `stage` (not the place behind them).
pub fn draw<D>(target: &mut D, stage: &Stage) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    let shade = stage.place.shadow();
    let dogs = [Some(stage.him), stage.her];
    for p in dogs.iter().flatten() {
        shadow(target, p, shade)?;
    }
    // The one further back first.
    let (first, second) = match stage.her {
        Some(her) if her.y < stage.him.y => (Some(her), Some(stage.him)),
        Some(her) => (Some(stage.him), Some(her)),
        None => (Some(stage.him), None),
    };
    for p in [first, second].iter().flatten() {
        pup(target, p)?;
    }
    for s in stage.props.iter().flatten() {
        prop(target, s)?;
    }
    Ok(())
}
