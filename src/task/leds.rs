//! The two LEDs: a breathing glow and a beating heart, plus a flash on each tap.
//!
//! The patterns give a brightness from 0 (off) to 255 (full) as the eye sees
//! it. [`duty`] turns that into how much of the time the LED must be on, and
//! [`Dither`] turns the duty into a fast on/off stream for the pins, which can
//! only be fully on or fully off.
use crate::state::{Model, Power, Scene};

/// Full brightness.
pub const FULL: u8 = 255;

/// Rises from 0 to `top` and back once every `period_ms`, easing in and out
/// like slow breathing.
pub fn breathe(t_ms: u32, period_ms: u32, top: u8) -> u8 {
    let period = period_ms.max(2);
    let half = period / 2;
    let p = t_ms % period;
    // A triangle wave 0 ..= 255 ..= 0, then smoothstep so the ends linger.
    let tri = if p < half {
        p * 255 / half
    } else {
        (period - p) * 255 / (period - half)
    };
    let x = tri.min(255);
    let smooth = x * x * (3 * 255 - 2 * x) / (255 * 255);
    scale(smooth, top)
}

/// One pulse: quick rise to `peak` over `rise_ms`, then a slower fade over
/// `fall_ms`. `t_ms` is the time since the pulse started.
fn pulse(t_ms: u32, peak: u8, rise_ms: u32, fall_ms: u32) -> u32 {
    let peak = u32::from(peak);
    if t_ms < rise_ms {
        peak * t_ms / rise_ms.max(1)
    } else if t_ms < rise_ms + fall_ms {
        let left = rise_ms + fall_ms - t_ms;
        // Fade along a curve: bright for a moment, then a soft tail.
        peak * left * left / (fall_ms * fall_ms).max(1)
    } else {
        0
    }
}

/// A heartbeat: a strong "lub" and a softer "dub", then a rest, once every
/// `period_ms`.
pub fn heartbeat(t_ms: u32, period_ms: u32) -> u8 {
    let p = t_ms % period_ms.max(1);
    let lub = pulse(p, FULL, 40, 180);
    let dub = p.checked_sub(230).map_or(0, |q| pulse(q, 170, 40, 220));
    u8::try_from(lub.max(dub).min(255)).unwrap_or(FULL)
}

fn scale(v: u32, top: u8) -> u8 {
    u8::try_from(v * u32::from(top) / 255).unwrap_or(top)
}

/// The pattern for a scene, `t_ms` milliseconds after the scene started, as
/// brightness. Returns `(green, red)`.
pub fn sample(scene: Scene, t_ms: u32) -> (u8, u8) {
    match scene {
        // A slow green breath while waiting to be opened.
        Scene::Intro => (breathe(t_ms, 3_000, FULL), 0),
        // A calm red heartbeat while reading the story.
        Scene::Story { .. } => (0, heartbeat(t_ms, 1_200)),
        // Drawing the heart: the beat speeds up with every dot traced, over
        // a soft green glow.
        Scene::Trace { hit } => {
            let dots = (hit & crate::trace::ALL).count_ones();
            (breathe(t_ms, 2_000, 90), heartbeat(t_ms, 1_100 - dots * 25))
        }
        // The finale: a calm heartbeat over a soft green glow.
        Scene::Finale { .. } => (breathe(t_ms, 4_000, 60), heartbeat(t_ms, 1_000)),
        // The heart races at the big question.
        Scene::Question => (0, heartbeat(t_ms, 650)),
        // Both at once: a happy heartbeat over a quick green breath.
        Scene::Celebration => (breathe(t_ms, 1_400, FULL), heartbeat(t_ms, 900)),
    }
}

/// How much of the time an LED must be on to look `brightness` bright. The
/// eye is far more sensitive at the dim end, so this squares the brightness
/// (a gamma of about 2). Anything above zero stays at least a little on.
pub fn duty(brightness: u8) -> u8 {
    let b = u32::from(brightness);
    if b == 0 {
        return 0;
    }
    u8::try_from(((b * b) / 255).max(1)).unwrap_or(FULL)
}

/// What the LEDs show right now, as duty: both full for a moment after a tap
/// on an object, otherwise the pattern for the current scene. Returns
/// `(green, red)`.
pub fn output(m: &Model) -> (u8, u8) {
    if m.power(m.now_ms) == Power::Asleep {
        // Asleep: a slow, faint green breath, like a sleeping laptop.
        return (duty(breathe(m.now_ms, 5_000, 70)), 0);
    }
    if m.now_ms < m.flash_until_ms {
        (FULL, FULL)
    } else {
        let (g, r) = sample(m.scene, m.now_ms.wrapping_sub(m.scene_start_ms));
        (duty(g), duty(r))
    }
}

/// Turns a duty from 0 to 255 into a stream of on/off steps that is on that
/// many 255ths of the time. Call [`Dither::step`] at a steady, fast rate (the
/// firmware uses 10 kHz).
///
/// Each step adds the duty to a running total and switches the LED on
/// whenever the total passes 255. The on-steps come spread out as evenly as
/// possible, so even dim levels switch fast enough not to flicker.
#[derive(Copy, Clone, Debug, Default)]
pub struct Dither {
    acc: [u16; 2],
}

impl Dither {
    /// Both LEDs off, nothing carried over.
    pub const fn new() -> Self {
        Self { acc: [0; 2] }
    }

    /// The next step for `(green, red)` at duty `levels`: true means on.
    pub fn step(&mut self, levels: (u8, u8)) -> (bool, bool) {
        let mut on = |i: usize, level: u8| {
            let total = self.acc[i] + u16::from(level);
            let lit = total >= 255;
            self.acc[i] = if lit { total - 255 } else { total };
            lit
        };
        (on(0, levels.0), on(1, levels.1))
    }
}

#[cfg(test)]
mod tests {
    use super::{breathe, duty, heartbeat, sample, Dither, FULL};
    use crate::common::testing::Rig;
    use crate::state::Scene;

    fn story() -> Scene {
        Scene::Story {
            chapter: 0,
            found: 0,
            shown: None,
        }
    }

    #[test]
    fn breathing_goes_from_dark_to_full_and_back() {
        assert_eq!(breathe(0, 3_000, FULL), 0);
        assert_eq!(breathe(1_500, 3_000, FULL), FULL);
        assert_eq!(breathe(3_000, 3_000, FULL), 0, "repeats");
        let quarter = breathe(750, 3_000, FULL);
        assert!(quarter > 60 && quarter < 200, "half way up: {quarter}");
        assert!(breathe(1_500, 3_000, 100) <= 100, "never above top");
    }

    #[test]
    fn breathing_rises_then_falls_smoothly() {
        let mut last = 0;
        for t in (0..=1_500).step_by(50) {
            let b = breathe(t, 3_000, FULL);
            assert!(b >= last, "rising at {t} ms: {b} after {last}");
            last = b;
        }
        for t in (1_500..=3_000).step_by(50) {
            let b = breathe(t, 3_000, FULL);
            assert!(b <= last, "falling at {t} ms: {b} after {last}");
            last = b;
        }
    }

    #[test]
    fn a_heartbeat_has_a_lub_a_dub_and_a_rest() {
        let lub = heartbeat(40, 1_200);
        let between = heartbeat(200, 1_200);
        let dub = heartbeat(270, 1_200);
        let rest = heartbeat(800, 1_200);
        assert_eq!(lub, FULL, "lub peaks at full");
        assert!(dub > 120 && dub < lub, "dub is softer: {dub}");
        assert!(between < dub, "a dip between the two: {between}");
        assert_eq!(rest, 0, "dark until the next beat");
        assert_eq!(heartbeat(1_200 + 40, 1_200), FULL, "repeats");
    }

    #[test]
    fn duty_squares_the_brightness() {
        assert_eq!(duty(0), 0);
        assert_eq!(duty(FULL), FULL);
        assert_eq!(duty(128), 64);
        assert_eq!(duty(3), 1, "dim but not off");
    }

    #[test]
    fn each_scene_has_its_own_pattern() {
        assert_eq!(
            sample(Scene::Intro, 1_500),
            (FULL, 0),
            "intro breathes green"
        );
        assert_eq!(sample(story(), 40), (0, FULL), "story beats red");
        assert_eq!(sample(story(), 800).1, 0);
        assert_eq!(sample(Scene::Question, 650 + 40), (0, FULL), "beats faster");
        let party = sample(Scene::Celebration, 40);
        assert_eq!(party.1, FULL, "celebration beats red...");
        assert!(
            sample(Scene::Celebration, 700).0 > 200,
            "...and breathes green"
        );
    }

    #[test]
    fn dither_is_on_for_the_right_share_of_the_time() {
        for level in [0u8, 1, 17, 64, 128, 200, 254, 255] {
            let mut d = Dither::new();
            let on = (0..255 * 4).filter(|_| d.step((level, 0)).0).count();
            assert_eq!(on, usize::from(level) * 4, "level {level}");
        }
    }

    #[test]
    fn dither_spreads_the_on_steps_out() {
        let mut d = Dither::new();
        let mut offs_in_a_row = 0;
        for i in 0..255 {
            if d.step((0, 128)).1 {
                offs_in_a_row = 0;
            } else {
                offs_in_a_row += 1;
            }
            assert!(
                offs_in_a_row < 3,
                "three offs in a row at half duty, step {i}"
            );
        }
    }

    #[test]
    fn the_effects_carry_the_pattern_for_the_current_scene() {
        let mut r = Rig::new();
        assert_eq!(
            r.tick(1_500).leds,
            (FULL, 0),
            "intro: green at the top of a breath"
        );
        assert_eq!(r.tick(3_000).leds, (0, 0), "and dark between breaths");
    }

    #[test]
    fn asleep_the_leds_only_breathe_faintly() {
        let mut r = Rig::new();
        let t = crate::state::SLEEP_AFTER_MS + 2_500;
        let (g, red) = r.tick(t).leds;
        assert!(g > 0 && g < 40, "faint green: {g}");
        assert_eq!(red, 0);
    }

    #[test]
    fn the_question_heart_beats_red() {
        let mut r = Rig::new();
        r.play_to_question();
        let t0 = r.m.now_ms;
        assert_eq!(r.tick(t0 + 40).leds, (0, FULL));
        assert_eq!(r.tick(t0 + 500).leds, (0, 0));
    }
}
