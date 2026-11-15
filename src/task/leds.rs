//! LED statemachine. The LEDs show a pattern for each scene, plus a flash on each tap.
use crate::state::{Model, Scene};

/// The pattern for a scene, `t_ms` milliseconds after the scene started.
/// Returns `(green, red)`.
pub fn sample(scene: Scene, t_ms: u32) -> (bool, bool) {
    match scene {
        // Slow blink: on for the first half of every second.
        Scene::Intro => (t_ms % 1000 < 500, false),
        // Heartbeat: two short pulses every 1.2 s.
        Scene::Story { .. } => {
            let p = t_ms % 1200;
            (p < 100 || (200..300).contains(&p), false)
        }
        // Nervous: the two LEDs alternate every 150 ms.
        Scene::Question => {
            let a = (t_ms / 150).is_multiple_of(2);
            (a, !a)
        }
        // Party: green, red, both, every 80 ms.
        Scene::Celebration => match (t_ms / 80) % 3 {
            0 => (true, false),
            1 => (false, true),
            _ => (true, true),
        },
    }
}

/// What the LEDs show right now: both on briefly after a tap on an object,
/// otherwise the pattern for the current scene.
pub fn output(m: &Model) -> (bool, bool) {
    if m.now_ms < m.flash_until_ms {
        (true, true)
    } else {
        sample(m.scene, m.now_ms.wrapping_sub(m.scene_start_ms))
    }
}

#[cfg(test)]
mod tests {
    use super::sample;
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
    fn intro_blinks_half_the_time() {
        assert_eq!(sample(Scene::Intro, 0), (true, false));
        assert_eq!(sample(Scene::Intro, 499), (true, false));
        assert_eq!(sample(Scene::Intro, 500), (false, false));
        assert_eq!(sample(Scene::Intro, 1000), (true, false));
    }

    #[test]
    fn story_has_a_double_pulse() {
        assert!(sample(story(), 50).0);
        assert!(!sample(story(), 150).0);
        assert!(sample(story(), 250).0);
        assert!(!sample(story(), 700).0);
        assert!(sample(story(), 1200 + 50).0, "pattern repeats");
        assert!(!sample(story(), 0).1, "red stays off");
    }

    #[test]
    fn question_alternates() {
        assert_eq!(sample(Scene::Question, 0), (true, false));
        assert_eq!(sample(Scene::Question, 150), (false, true));
        assert_eq!(sample(Scene::Question, 300), (true, false));
    }

    #[test]
    fn celebration_cycles_through_three_states() {
        assert_eq!(sample(Scene::Celebration, 0), (true, false));
        assert_eq!(sample(Scene::Celebration, 80), (false, true));
        assert_eq!(sample(Scene::Celebration, 160), (true, true));
        assert_eq!(sample(Scene::Celebration, 240), (true, false));
    }

    #[test]
    fn the_effects_carry_the_pattern_for_the_current_scene() {
        let mut r = Rig::new();
        assert_eq!(
            r.tick(100).leds,
            (true, false),
            "intro: first half-second is on"
        );
        assert_eq!(r.tick(600).leds, (false, false));
    }

    #[test]
    fn the_question_leds_are_opposite_and_swap_every_150_ms() {
        let mut r = Rig::new();
        r.play_to_question();
        let t0 = r.m.now_ms;
        let a = r.tick(t0).leds;
        let b = r.tick(t0 + 150).leds;
        assert_eq!(a.0, !a.1);
        assert_eq!(b, (a.1, a.0));
    }
}
