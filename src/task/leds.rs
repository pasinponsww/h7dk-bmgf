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
