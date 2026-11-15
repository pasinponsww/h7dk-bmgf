//! The clock task: idle hints, the "Next" unlock, and the auto-restart.

use crate::event::Effects;
use crate::state::{Model, Scene, IDLE_RESET_MS, NUDGE_AFTER_MS, NUDGE_EVERY_MS, UNLOCK_AFTER_MS};
use crate::story::{full_mask, Story};

use super::touch::first_unfound;

pub(crate) fn on_tick<S: Story>(m: &mut Model, story: &S, fx: &mut Effects) {
    let idle = m.now_ms.wrapping_sub(m.last_touch_ms);
    match m.scene {
        Scene::Story { chapter, found, .. } => {
            let n = story.hotspot_count(usize::from(chapter));
            let all = found == full_mask(n);
            if !all
                && idle > NUDGE_AFTER_MS
                && m.now_ms.wrapping_sub(m.last_nudge_ms) > NUDGE_EVERY_MS
            {
                m.last_nudge_ms = m.now_ms;
                fx.nudge = first_unfound(found, n);
            }
            if idle > UNLOCK_AFTER_MS && !m.skip_ok {
                m.skip_ok = true;
                fx.redraw = true;
            }
        }
        Scene::Celebration => {
            if idle > IDLE_RESET_MS {
                m.go(Scene::Intro);
                fx.scene_changed = true;
                fx.redraw = true;
            }
        }
        Scene::Intro | Scene::Question => {}
    }
}

#[cfg(test)]
mod tests {
    use crate::common::testing::{story_state, Rig};
    use crate::state::{Scene, IDLE_RESET_MS, NUDGE_AFTER_MS, UNLOCK_AFTER_MS};

    #[test]
    fn idle_nudges_every_three_seconds_then_unlocks_next() {
        let mut r = Rig::new();
        r.tap(1, 1);
        assert_eq!(r.tick(NUDGE_AFTER_MS - 1).nudge, None);
        assert_eq!(r.tick(NUDGE_AFTER_MS + 100).nudge, Some(0));
        assert_eq!(
            r.tick(NUDGE_AFTER_MS + 600).nudge,
            None,
            "not again right away"
        );
        assert_eq!(r.tick(2 * NUDGE_AFTER_MS + 200).nudge, Some(0));
        assert!(!r.m.skip_ok);

        let fx = r.tick(UNLOCK_AFTER_MS + 100);
        assert!(r.m.skip_ok && fx.redraw);
        let fx = r.tap_next();
        assert!(fx.scene_changed, "with nothing found, Next now works");
        assert_eq!(story_state(&r.m).0, 1);
    }

    #[test]
    fn touching_resets_the_idle_timer() {
        let mut r = Rig::new();
        r.tap(1, 1);
        r.tick(2_900);
        r.tap(5, 5);
        assert_eq!(r.tick(5_000).nudge, None);
        assert_eq!(r.tick(2_900 + NUDGE_AFTER_MS + 100).nudge, Some(0));
    }

    #[test]
    fn the_intro_and_question_never_nudge() {
        let mut r = Rig::new();
        assert_eq!(r.tick(20_000).nudge, None);
        r.play_to_question();
        let t = r.m.now_ms + 20_000;
        assert_eq!(r.tick(t).nudge, None);
    }

    #[test]
    fn celebration_returns_to_the_intro_after_a_minute_idle() {
        let mut r = Rig::new();
        r.play_to_question();
        let yes = r.m.yes_rect();
        r.tap_rect(yes);
        assert_eq!(r.m.scene, Scene::Celebration);
        r.tick(IDLE_RESET_MS - 1);
        assert_eq!(r.m.scene, Scene::Celebration);
        let fx = r.tick(IDLE_RESET_MS + 1);
        assert_eq!(r.m.scene, Scene::Intro);
        assert!(fx.scene_changed);
    }
}
