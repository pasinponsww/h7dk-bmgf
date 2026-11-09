//! The touch task: a finger went down. What it does depends on the screen.

use geometry::place::{farthest_free_spot, Placement};
use geometry::Point;
use rng::Rng;

use crate::event::Effects;
use crate::state::{
    Model, Scene, DODGE_RADIUS, FLASH_MS, GIVE_UP_W, JUMP_TRIES, MAX_DODGES, NEXT_BUTTON, NO_AREA,
    SCREEN_W, YES_PAD,
};
use crate::story::{full_mask, Story};

/// Index of the first object not yet tapped.
pub(crate) fn first_unfound(found: u8, n: usize) -> Option<u8> {
    (0..n.min(8))
        .find(|i| found & (1u8 << i) == 0)
        .and_then(|i| u8::try_from(i).ok())
}

pub(crate) fn on_down<S: Story, R: Rng>(
    m: &mut Model,
    story: &S,
    rng: &mut R,
    p: Point,
    fx: &mut Effects,
) {
    m.last_touch_ms = m.now_ms;
    fx.ripple = Some(p);
    match m.scene {
        Scene::Intro => {
            m.go(Scene::Story {
                chapter: 0,
                found: 0,
                shown: None,
            });
            fx.scene_changed = true;
            fx.redraw = true;
        }
        Scene::Story { chapter, found, .. } => story_touch(m, story, chapter, found, p, fx),
        Scene::Question => question_touch(m, rng, p, fx),
        Scene::Celebration => {}
    }
}

fn story_touch<S: Story>(
    m: &mut Model,
    story: &S,
    chapter: u8,
    found: u8,
    p: Point,
    fx: &mut Effects,
) {
    let ch = usize::from(chapter);
    let n = story.hotspot_count(ch);
    let full = full_mask(n);

    // Did she tap an object?
    for i in 0..n.min(8) {
        if story.hotspot_rect(ch, i).contains(p) {
            let Ok(idx) = u8::try_from(i) else { return };
            let new_found = found | (1u8 << i);
            m.scene = Scene::Story {
                chapter,
                found: new_found,
                shown: Some(idx),
            };
            m.flash_until_ms = m.now_ms.wrapping_add(FLASH_MS);
            fx.reveal = Some(idx);
            fx.redraw = true;
            fx.chapter_complete = found != full && new_found == full;
            return;
        }
    }

    // Did she tap "Next"?
    if NEXT_BUTTON.contains(p) {
        if found != full && !m.skip_ok {
            fx.nudge = first_unfound(found, n);
            return;
        }
        let next = ch + 1;
        if next >= story.chapter_count() {
            m.go(Scene::Question);
        } else {
            m.go(Scene::Story {
                chapter: u8::try_from(next).unwrap_or(u8::MAX),
                found: 0,
                shown: None,
            });
        }
        fx.scene_changed = true;
        fx.redraw = true;
    }
    // Anything else is a miss: the ripple is the only feedback.
}

fn question_touch<R: Rng>(m: &mut Model, rng: &mut R, p: Point, fx: &mut Effects) {
    // YES always wins, wherever and whenever.
    if m.yes_rect().contains(p) {
        celebrate(m, fx);
        return;
    }
    // Once NO has given up it stops moving and becomes "Okay, yes".
    if m.no_gave_up() {
        if m.no_rect.contains(p) {
            celebrate(m, fx);
        }
        return;
    }
    // A touch near NO makes it jump before the press can land.
    let radius = i32::from(DODGE_RADIUS);
    if m.no_rect.dist_sq(p) < radius * radius {
        m.dodges += 1;
        let place = Placement {
            area: NO_AREA,
            avoid: m.yes_rect(),
            pad: YES_PAD,
            tries: JUMP_TRIES,
        };
        m.no_rect = farthest_free_spot(m.no_rect, &place, &mut || rng.next_u32());
        if m.dodges >= MAX_DODGES {
            m.no_rect.w = GIVE_UP_W;
            m.no_rect.x = m.no_rect.x.min(SCREEN_W - 10 - GIVE_UP_W);
        }
        fx.dodged = true;
        fx.redraw = true;
    }
}

fn celebrate(m: &mut Model, fx: &mut Effects) {
    m.go(Scene::Celebration);
    fx.scene_changed = true;
    fx.redraw = true;
}
