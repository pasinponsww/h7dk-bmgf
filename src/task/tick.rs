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
