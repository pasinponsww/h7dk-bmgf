//! Moments: what the two dogs do while a line of the story is up.
//!
//! Every object in a chapter can have a moment: a place (a background, see
//! `ui::places`) and an act. [`stage`] turns a moment and the time since it
//! began into where each dog stands, which way it faces, whether it is
//! mid-step, its face, what it wears, and which props float where. The
//! painter draws that; nothing is stored between frames.
//!
//! The dogs walk about like 2D characters in a 3D world: a dog lower on the
//! screen is nearer, drawn in front, and walks toward the camera (front),
//! away from it (back) or sideways (side, mirrored for left).

use ui::places::Place;

/// Which way a dog faces.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Facing {
    /// Toward the camera.
    Front,
    /// Away from the camera.
    Back,
    /// To the left of the screen.
    Left,
    /// To the right of the screen.
    Right,
}

/// The eyes, from the front (or the side).
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Eyes {
    /// Round and open (with lashes on her).
    Open,
    /// A blink.
    Closed,
    /// Huge and amazed.
    Wow,
    /// Hearts.
    Heart,
    /// Bigger hearts, for a throb.
    HeartBig,
    /// Happy arches.
    Happy,
    /// Half shut, relaxed.
    Chill,
}

/// The mouth, from the front.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Mouth {
    /// A little smile.
    Smile,
    /// Wide open.
    Wow,
    /// A big grin.
    Grin,
    /// Laughing.
    Laugh,
}

/// Things a dog can wear, as bits.
pub mod wear {
    /// Round glasses.
    pub const GLASSES: u8 = 1;
    /// Sunglasses.
    pub const SUNGLASSES: u8 = 1 << 1;
    /// Red and blue movie glasses.
    pub const GLASSES_3D: u8 = 1 << 2;
    /// A black hat.
    pub const BLACK_HAT: u8 = 1 << 3;
    /// A straw hat.
    pub const STRAW_HAT: u8 = 1 << 4;
}

/// One dog at one moment.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Pup {
    /// Middle of its feet on the screen.
    pub x: i32,
    /// Its feet on the screen: lower is nearer.
    pub y: i32,
    /// Which way it faces.
    pub facing: Facing,
    /// Mid-step (the walking frame).
    pub step: bool,
    /// Pixels off the ground.
    pub lift: i32,
    /// Its eyes.
    pub eyes: Eyes,
    /// Its mouth.
    pub mouth: Mouth,
    /// Pink cheeks.
    pub blush: bool,
    /// What it wears ([`wear`] bits).
    pub wear: u8,
    /// Her: lashes and a bow.
    pub her: bool,
}

/// A prop.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Prop {
    Thought,
    Heart,
    Laptop,
    Sun,
    Popcorn,
    Gelato,
    Boba,
    Sushi,
    Ramen,
    Phone,
    Matcha,
    Note,
    Sparkle,
}

/// A prop floating somewhere: its top-left corner and its pixel size.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Shown {
    pub prop: Prop,
    pub x: i32,
    pub y: i32,
    pub scale: u32,
}

/// The most props one moment shows at once.
pub const MAX_PROPS: usize = 6;

/// Everything on the stage at one moment.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Stage {
    /// The background.
    pub place: Place,
    /// Him.
    pub him: Pup,
    /// Her, if she is in this moment.
    pub her: Option<Pup>,
    /// Props, in no particular order.
    pub props: [Option<Shown>; MAX_PROPS],
}

/// What the dogs do in a moment.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Act {
    /// Both wander about.
    Wander,
    /// His eyes go huge and his mouth drops: she walks in.
    Wow,
    /// Heart eyes, throbbing, while she poses.
    Smitten,
    /// He dreams of her in a thought cloud.
    Think,
    /// He watches her, bouncing with excitement.
    Excited,
    /// A heart beating wildly in front of him.
    Heartbeat,
    /// Laughing in glasses and a black hat, music notes about.
    Laugh,
    /// The biggest smile.
    Grin,
    /// Straw hat and sunglasses under the sun.
    Summer,
    /// Just chilling.
    Chill,
    /// Looking into each other's eyes.
    Gaze,
    /// 3D glasses and popcorn.
    Movie,
    /// She jumps at the scary film, he laughs, she smiles.
    Scared,
    /// He feeds her gelato.
    Gelato,
    /// A pink drink each, then sushi.
    Treats,
    /// Looking at paintings, from behind.
    Museum,
    /// Walking side by side.
    Stroll,
    /// A hug.
    Hug,
    /// Ramen together.
    Ramen,
    /// Matcha each.
    Matcha,
    /// He runs zoomies round her.
    Zoomies,
    /// Heart eyes both, a big heart between.
    Love,
}

impl Act {
    /// Every act, in the order of [`Act::NAMES`].
    pub const ALL: [Act; 22] = [
        Act::Wander,
        Act::Wow,
        Act::Smitten,
        Act::Think,
        Act::Excited,
        Act::Heartbeat,
        Act::Laugh,
        Act::Grin,
        Act::Summer,
        Act::Chill,
        Act::Gaze,
        Act::Movie,
        Act::Scared,
        Act::Gelato,
        Act::Treats,
        Act::Museum,
        Act::Stroll,
        Act::Hug,
        Act::Ramen,
        Act::Matcha,
        Act::Zoomies,
        Act::Love,
    ];
    /// The names a story file may use.
    pub const NAMES: [&'static str; 22] = [
        "wander",
        "wow",
        "smitten",
        "think",
        "excited",
        "heartbeat",
        "laugh",
        "grin",
        "summer",
        "chill",
        "gaze",
        "movie",
        "scared",
        "gelato",
        "treats",
        "museum",
        "stroll",
        "hug",
        "ramen",
        "matcha",
        "zoomies",
        "love",
    ];

    /// The act called `name`, if there is one.
    pub fn from_name(name: &str) -> Option<Act> {
        Self::NAMES
            .iter()
            .position(|n| *n == name)
            .map(|i| Self::ALL[i])
    }
}

/// A moment: a place and an act.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Moment {
    pub place: Place,
    pub act: Act,
}

impl Moment {
    /// The usual: both dogs wandering on pink.
    pub const IDLE: Moment = Moment {
        place: Place::Pink,
        act: Act::Wander,
    };
}

// ---------------------------------------------------------------- helpers

/// The stage's floor: where feet may go.
const LEFT: i32 = 56;
const RIGHT: i32 = 226;
const NEAR: i32 = 204;
const FAR: i32 = 172;

fn hash(mut v: u32) -> u32 {
    v ^= v >> 16;
    v = v.wrapping_mul(0x7feb_352d);
    v ^= v >> 15;
    v = v.wrapping_mul(0x846c_a68b);
    v ^ (v >> 16)
}

fn i(v: u32) -> i32 {
    i32::try_from(v).unwrap_or(i32::MAX)
}

/// 0 up to `peak` and back, once every `period` ms.
fn bounce(t: u32, period: u32, peak: i32) -> i32 {
    let p = t % period.max(2);
    let half = period.max(2) / 2;
    let up = if p < half { p } else { period - p };
    i(up) * peak / i(half)
}

/// A dog standing still, facing the camera.
const fn stand(x: i32, y: i32, her: bool) -> Pup {
    Pup {
        x,
        y,
        facing: Facing::Front,
        step: false,
        lift: 0,
        eyes: Eyes::Open,
        mouth: Mouth::Smile,
        blush: her,
        wear: 0,
        her,
    }
}

/// Blinks now and then.
fn blink(eyes: Eyes, t: u32, seed: u32) -> Eyes {
    if eyes == Eyes::Open && (t + seed * 1_300) % 3_600 < 130 {
        Eyes::Closed
    } else {
        eyes
    }
}

/// Facing for a move of `(dx, dy)`.
fn facing_of(dx: i32, dy: i32) -> Facing {
    if dx.abs() >= dy.abs() * 2 {
        if dx < 0 {
            Facing::Left
        } else {
            Facing::Right
        }
    } else if dy > 0 {
        Facing::Front
    } else {
        Facing::Back
    }
}

/// A dog walking from `a` to `b`, `p` thousandths of the way, `t` ms in.
fn walking(a: (i32, i32), b: (i32, i32), p: i32, t: u32, her: bool) -> Pup {
    let p = p.clamp(0, 1_000);
    let x = a.0 + (b.0 - a.0) * p / 1_000;
    let y = a.1 + (b.1 - a.1) * p / 1_000;
    let moving = p < 1_000 && a != b;
    Pup {
        facing: if moving {
            facing_of(b.0 - a.0, b.1 - a.1)
        } else {
            Facing::Front
        },
        step: moving && (t / 150) % 2 == 1,
        ..stand(x, y, her)
    }
}

/// Wandering: walk to a spot, look about, walk on. Each dog has its own
/// `seed` so they go their own ways.
fn wander(t: u32, seed: u32, her: bool) -> Pup {
    const LEG: u32 = 2_800;
    const WALK: u32 = 1_300;
    let spot = |n: u32| {
        let h = hash(n.wrapping_mul(0x9e37_79b9) ^ seed);
        let x = LEFT + i(h % u32::try_from(RIGHT - LEFT).unwrap_or(1));
        let y = FAR + i(h / 1_000 % u32::try_from(NEAR - FAR).unwrap_or(1));
        (x, y)
    };
    let n = t / LEG;
    let into = t % LEG;
    let p = i(into.min(WALK) * 1_000 / WALK);
    let mut pup = walking(spot(n), spot(n + 1), p, t, her);
    pup.eyes = blink(pup.eyes, t, seed);
    pup
}

/// Props from a list, the rest empty.
fn props<const N: usize>(list: [Shown; N]) -> [Option<Shown>; MAX_PROPS] {
    let mut out = [None; MAX_PROPS];
    for (slot, p) in out.iter_mut().zip(list) {
        *slot = Some(p);
    }
    out
}

const fn shown(prop: Prop, x: i32, y: i32, scale: u32) -> Shown {
    Shown { prop, x, y, scale }
}

/// A sparkle that blinks on and off at `(x, y)`.
fn sparkle(t: u32, x: i32, y: i32, phase: u32) -> Shown {
    let big = (t + phase) % 900 < 450;
    shown(Prop::Sparkle, x, y, if big { 3 } else { 2 })
}

/// Hearts rising from `(x, y)`, one every so often.
fn rising_heart(t: u32, x: i32, y: i32, phase: u32) -> Shown {
    let p = i((t + phase) % 1_800);
    shown(Prop::Heart, x + (p / 300 % 2) * 4, y - p * 50 / 1_800, 2)
}

// ------------------------------------------------------------------ acts

/// Everything on the stage for `moment`, `t` ms after it began.
pub fn stage(moment: &Moment, t: u32) -> Stage {
    let place = moment.place;
    let mut s = Stage {
        place,
        him: stand(100, 196, false),
        her: None,
        props: [None; MAX_PROPS],
    };
    match moment.act {
        Act::Wander => {
            s.him = wander(t, 0x51, false);
            s.her = Some(wander(t + 900, 0xa7, true));
        }
        Act::Wow => {
            // He jumps at the sight of her coming in from the right.
            s.him.eyes = Eyes::Wow;
            s.him.mouth = Mouth::Wow;
            s.him.lift = if t < 500 { bounce(t, 500, 18) } else { 0 };
            let walk_in = i(t.min(1_800) * 1_000 / 1_800);
            let mut her = walking((260, 186), (196, 186), walk_in, t, true);
            if walk_in == 1_000 {
                her.eyes = blink(Eyes::Open, t, 3);
            }
            s.her = Some(her);
            s.props = props([sparkle(t, 50, 100, 0), sparkle(t, 140, 96, 450)]);
        }
        Act::Smitten => {
            s.him.eyes = if (t / 260).is_multiple_of(2) {
                Eyes::HeartBig
            } else {
                Eyes::Heart
            };
            s.him.blush = true;
            s.him.lift = bounce(t, 900, 2);
            let mut her = stand(196, 188, true);
            her.eyes = Eyes::Happy;
            her.mouth = Mouth::Grin;
            s.her = Some(her);
            s.props = props([
                sparkle(t, 162, 104, 0),
                sparkle(t, 232, 96, 300),
                sparkle(t, 214, 150, 600),
                rising_heart(t, 120, 120, 0),
            ]);
        }
        Act::Think => {
            s.him = stand(92, 198, false);
            s.him.eyes = blink(Eyes::Open, t, 1);
            let mut her = stand(206, 182, true);
            her.eyes = blink(Eyes::Open, t, 5);
            s.her = Some(her);
            let bob = bounce(t, 1_600, 4);
            s.props = props([shown(Prop::Thought, 98, 56 + bob, 3)]);
        }
        Act::Excited => {
            s.him = stand(108, 196, false);
            s.him.facing = Facing::Right;
            s.him.step = (t / 110).is_multiple_of(2);
            s.him.lift = bounce(t, 420, 8);
            let mut her = stand(204, 188, true);
            her.eyes = blink(Eyes::Open, t, 7);
            s.her = Some(her);
            s.props = props([rising_heart(t, 150, 140, 0), rising_heart(t, 136, 140, 900)]);
        }
        Act::Heartbeat => {
            s.him = stand(92, 198, false);
            s.him.eyes = Eyes::Wow;
            s.him.mouth = Mouth::Wow;
            // Panting: a quick little bob.
            s.him.lift = bounce(t, 260, 3);
            let beat = (t % 520) < 130 || ((t % 520) >= 220 && (t % 520) < 330);
            let (size, off) = if beat { (4, 0) } else { (3, 6) };
            s.props = props([shown(Prop::Heart, 140 + off, 112 + off, size)]);
            let mut her = stand(214, 182, true);
            her.eyes = blink(Eyes::Open, t, 2);
            s.her = Some(her);
        }
        Act::Laugh => {
            s.him.eyes = Eyes::Happy;
            s.him.mouth = Mouth::Laugh;
            s.him.wear = wear::GLASSES | wear::BLACK_HAT;
            s.him.lift = bounce(t, 360, 5);
            let mut her = stand(196, 188, true);
            her.eyes = Eyes::Happy;
            her.mouth = Mouth::Laugh;
            her.lift = bounce(t + 180, 360, 4);
            s.her = Some(her);
            let note = |phase: u32, x: i32| {
                let p = i((t + phase) % 2_000);
                shown(Prop::Note, x + (p / 250 % 2) * 4, 140 - p * 70 / 2_000, 3)
            };
            s.props = props([note(0, 40), note(700, 150), note(1_400, 240)]);
        }
        Act::Grin => {
            s.him.eyes = Eyes::Happy;
            s.him.mouth = Mouth::Grin;
            s.him.lift = bounce(t, 1_200, 3);
            let mut her = stand(200, 186, true);
            her.eyes = Eyes::Happy;
            her.mouth = Mouth::Smile;
            s.her = Some(her);
            s.props = props([sparkle(t, 60, 104, 0), sparkle(t, 150, 98, 450)]);
        }
        Act::Summer => {
            s.him = stand(120, 198, false);
            s.him.wear = wear::STRAW_HAT | wear::SUNGLASSES;
            s.him.mouth = Mouth::Grin;
            s.him.lift = bounce(t, 1_400, 2);
            let pulse = (t / 400) % 2;
            s.props = props([shown(
                Prop::Sun,
                206 - i(pulse) * 4,
                70 - i(pulse) * 4,
                3 + pulse,
            )]);
        }
        Act::Chill => {
            s.him = stand(120, 198, false);
            s.him.eyes = Eyes::Chill;
            s.him.mouth = Mouth::Smile;
            // A slow sway, side to side.
            let sway = bounce(t, 3_000, 6) - 3;
            s.him.x += sway;
        }
        Act::Gaze => {
            let mut him = stand(112, 194, false);
            him.facing = Facing::Right;
            let mut her = stand(184, 194, true);
            her.facing = Facing::Left;
            him.lift = bounce(t, 1_600, 2);
            her.lift = bounce(t + 800, 1_600, 2);
            s.him = him;
            s.her = Some(her);
            s.props = props([rising_heart(t, 140, 130, 0), rising_heart(t, 150, 130, 900)]);
        }
        Act::Movie => {
            s.him = stand(96, 196, false);
            s.him.wear = wear::GLASSES_3D;
            s.him.mouth = Mouth::Wow;
            let mut her = stand(190, 196, true);
            her.wear = wear::GLASSES_3D;
            her.mouth = Mouth::Smile;
            s.her = Some(her);
            s.props = props([shown(Prop::Popcorn, 124, 164, 3)]);
        }
        Act::Scared => {
            // She jumps at the film, he laughs, then she smiles.
            let mut her = stand(190, 196, true);
            if t % 6_000 < 2_600 {
                her.eyes = Eyes::Wow;
                her.mouth = Mouth::Wow;
                her.x += if (t / 60).is_multiple_of(2) { -2 } else { 2 };
                her.lift = if t % 6_000 < 300 { 10 } else { 0 };
            } else {
                her.eyes = Eyes::Happy;
                her.mouth = Mouth::Grin;
            }
            s.her = Some(her);
            s.him = stand(96, 196, false);
            s.him.eyes = Eyes::Happy;
            s.him.mouth = Mouth::Laugh;
            s.him.lift = bounce(t, 380, 4);
            s.props = props([shown(Prop::Popcorn, 124, 164, 3)]);
        }
        Act::Gelato => {
            let mut him = stand(110, 196, false);
            him.facing = Facing::Right;
            let mut her = stand(196, 196, true);
            her.eyes = Eyes::Happy;
            her.mouth = Mouth::Grin;
            her.blush = true;
            s.him = him;
            s.her = Some(her);
            let reach = bounce(t, 1_600, 8);
            s.props = props([
                shown(Prop::Gelato, 132 + reach, 130, 3),
                rising_heart(t, 214, 110, 0),
            ]);
        }
        Act::Treats => {
            s.him = stand(108, 196, false);
            let mut her = stand(188, 196, true);
            her.eyes = Eyes::Happy;
            s.her = Some(her);
            // Pink drinks first, then sushi to share.
            if t % 8_000 < 4_000 {
                s.props = props([
                    shown(Prop::Boba, 58, 150, 3),
                    shown(Prop::Boba, 220, 150, 3),
                ]);
            } else {
                s.him.mouth = Mouth::Grin;
                s.props = props([shown(Prop::Sushi, 126, 170, 3)]);
            }
        }
        Act::Museum => {
            let drift = i(t % 6_000) * 30 / 6_000;
            let mut him = stand(76 + drift, 190, false);
            him.facing = Facing::Back;
            him.step = (t / 220).is_multiple_of(2);
            let mut her = stand(166 + drift, 190, true);
            her.facing = Facing::Back;
            her.step = (t / 220) % 2 == 1;
            s.him = him;
            s.her = Some(her);
            s.props = props([sparkle(t, 120, 60, 0), sparkle(t, 30, 40, 450)]);
        }
        Act::Stroll => {
            // Side by side across the street, then round again.
            let p = i(t % 5_000) * 1_000 / 5_000;
            let x = -40 + p * 320 / 1_000;
            let mut him = walking((x, 196), (x + 1, 196), 0, t, false);
            him.facing = Facing::Right;
            him.step = (t / 150).is_multiple_of(2);
            let mut her = him;
            her.x = x + 60;
            her.y = 190;
            her.her = true;
            her.blush = true;
            her.step = !him.step;
            s.him = him;
            s.her = Some(her);
        }
        Act::Hug => {
            let mut him = stand(132, 196, false);
            him.facing = Facing::Right;
            him.eyes = Eyes::Closed;
            let mut her = stand(170, 196, true);
            her.facing = Facing::Left;
            her.eyes = Eyes::Closed;
            s.him = him;
            s.her = Some(her);
            s.props = props([
                rising_heart(t, 150, 120, 0),
                rising_heart(t, 140, 120, 600),
                rising_heart(t, 160, 120, 1_200),
            ]);
        }
        Act::Ramen => {
            s.him = stand(94, 196, false);
            s.him.eyes = Eyes::Happy;
            let mut her = stand(198, 196, true);
            her.eyes = Eyes::Happy;
            s.her = Some(her);
            let steam = bounce(t, 1_000, 3);
            s.props = props([
                shown(Prop::Ramen, 120, 168 - steam, 3),
                sparkle(t, 150, 140, 0),
            ]);
        }
        Act::Matcha => {
            s.him = stand(100, 196, false);
            s.him.mouth = Mouth::Grin;
            let mut her = stand(194, 196, true);
            her.eyes = Eyes::Happy;
            s.her = Some(her);
            let sip = bounce(t, 2_000, 4);
            s.props = props([
                shown(Prop::Matcha, 52, 150 - sip, 3),
                shown(Prop::Matcha, 232, 150 - (4 - sip), 3),
            ]);
        }
        Act::Zoomies => {
            // Round and round her, then a happy flop.
            let mut her = stand(150, 186, true);
            her.eyes = Eyes::Happy;
            her.mouth = Mouth::Laugh;
            her.lift = bounce(t, 500, 3);
            s.her = Some(her);
            let lap = 1_600u32;
            let round = t % 6_400;
            if round < lap * 3 {
                let a = round % lap;
                // An oval round her: the four sides of a lap.
                let q = a * 4 / lap;
                let f = i(a % (lap / 4)) * 1_000 / i(lap / 4);
                let corners = [(64, 200), (236, 200), (236, 170), (64, 170)];
                let from = corners[q as usize];
                let to = corners[(q as usize + 1) % 4];
                let mut him = walking(from, to, f, t, false);
                him.step = (t / 70).is_multiple_of(2);
                him.mouth = Mouth::Laugh;
                him.eyes = Eyes::Happy;
                s.him = him;
            } else {
                s.him = stand(150, 204, false);
                s.him.eyes = Eyes::Happy;
                s.him.mouth = Mouth::Laugh;
                s.him.lift = bounce(round - lap * 3, 400, 14);
            }
        }
        Act::Love => {
            s.him = stand(100, 196, false);
            s.him.eyes = if (t / 300).is_multiple_of(2) {
                Eyes::HeartBig
            } else {
                Eyes::Heart
            };
            s.him.blush = true;
            let mut her = stand(196, 196, true);
            her.eyes = s.him.eyes;
            s.her = Some(her);
            let beat = (t / 300).is_multiple_of(2);
            s.props = props([
                shown(Prop::Heart, 130, 120, if beat { 4 } else { 3 }),
                sparkle(t, 60, 100, 0),
                sparkle(t, 236, 100, 450),
            ]);
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_act_has_a_name_that_finds_it() {
        for (a, n) in Act::ALL.iter().zip(Act::NAMES) {
            assert_eq!(Act::from_name(n), Some(*a));
        }
        assert_eq!(Act::from_name("dance"), None);
    }

    #[test]
    fn wandering_dogs_stay_on_the_floor_and_face_where_they_go() {
        for t in (0..40_000).step_by(37) {
            let s = stage(&Moment::IDLE, t);
            for p in [s.him, s.her.unwrap()] {
                assert!((LEFT..=RIGHT).contains(&p.x), "x {} at {t}", p.x);
                assert!((FAR..=NEAR).contains(&p.y), "y {} at {t}", p.y);
            }
        }
        // While walking, the facing follows the move.
        let a = stage(&Moment::IDLE, 100).him;
        let b = stage(&Moment::IDLE, 600).him;
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        if dx != 0 || dy != 0 {
            assert_eq!(b.facing, facing_of(dx, dy));
        }
    }

    #[test]
    fn the_wow_moment_drops_his_jaw_as_she_walks_in() {
        let m = Moment {
            place: Place::Club,
            act: Act::Wow,
        };
        let start = stage(&m, 100);
        assert_eq!((start.him.eyes, start.him.mouth), (Eyes::Wow, Mouth::Wow));
        let her = start.her.unwrap();
        assert_eq!(her.facing, Facing::Left, "walking in");
        assert!(her.her);
        let later = stage(&m, 3_000).her.unwrap();
        assert_eq!(later.facing, Facing::Front, "and stops to face us");
        assert!(later.x < her.x);
    }

    #[test]
    fn heart_eyes_throb_and_hearts_beat() {
        let m = Moment {
            place: Place::Pink,
            act: Act::Smitten,
        };
        assert_ne!(stage(&m, 0).him.eyes, stage(&m, 260).him.eyes);
        let b = Moment {
            place: Place::Pink,
            act: Act::Heartbeat,
        };
        let size = |t| stage(&b, t).props[0].unwrap().scale;
        assert_ne!(size(50), size(170), "the heart beats");
    }

    #[test]
    fn they_look_at_each_other_and_hug_face_to_face() {
        for act in [Act::Gaze, Act::Hug] {
            let s = stage(
                &Moment {
                    place: Place::Night,
                    act,
                },
                500,
            );
            assert_eq!(s.him.facing, Facing::Right, "{act:?}");
            assert_eq!(s.her.unwrap().facing, Facing::Left, "{act:?}");
        }
    }

    #[test]
    fn zoomies_go_round_her_then_flop() {
        let m = Moment {
            place: Place::Pink,
            act: Act::Zoomies,
        };
        let mut seen = [false; 4];
        for t in (0..1_600).step_by(50) {
            let k = match stage(&m, t).him.facing {
                Facing::Front => 0,
                Facing::Back => 1,
                Facing::Left => 2,
                Facing::Right => 3,
            };
            seen[k] = true;
        }
        let ways = seen.iter().filter(|&&b| b).count();
        assert!(ways >= 3, "running round: {seen:?}");
        assert!(stage(&m, 4_900).him.lift > 0, "a happy flop");
    }

    #[test]
    fn every_act_keeps_its_props_within_the_limit() {
        for act in Act::ALL {
            for t in (0..10_000).step_by(250) {
                let s = stage(
                    &Moment {
                        place: Place::Pink,
                        act,
                    },
                    t,
                );
                assert!(s.props.iter().flatten().count() <= MAX_PROPS);
            }
        }
    }
}
