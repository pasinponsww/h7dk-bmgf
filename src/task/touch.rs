//! The touch task: a finger went down. What it does depends on the screen.

use geometry::place::{farthest_free_spot, Placement};
use geometry::Point;
use rng::Rng;

use crate::event::Effects;
use crate::state::{
    Model, Scene, DODGE_RADIUS, FLASH_MS, GIVE_UP_W, JUMP_TRIES, MAX_DODGES, NEXT_BUTTON, NO_AREA,
    RESTART_BUTTON, SCREEN_W, YES_PAD,
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
        Scene::Celebration => {
            if RESTART_BUTTON.contains(p) {
                restart(m, fx);
            }
        }
    }
}

/// Back to the intro, as if the board had just started.
pub(crate) fn restart(m: &mut Model, fx: &mut Effects) {
    m.go(Scene::Intro);
    fx.scene_changed = true;
    fx.redraw = true;
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

#[cfg(test)]
mod tests {
    use crate::common::testing::{story_state, Fake, Rig, CH0};
    use crate::state::{Scene, FLASH_MS, GIVE_UP_W, INITIAL_NO, MAX_DODGES, RESTART_BUTTON};
    use crate::{update, Event};
    use geometry::Point;

    #[test]
    fn any_touch_on_the_intro_opens_the_story() {
        let mut r = Rig::new();
        let fx = r.tap(240, 136);
        assert_eq!(story_state(&r.m), (0, 0, None));
        assert!(fx.scene_changed && fx.redraw);
        assert_eq!(fx.ripple, Some(Point::new(240, 136)));
    }

    #[test]
    fn tapping_objects_reveals_lines_and_flashes_both_leds() {
        let mut r = Rig::new();
        r.tap(1, 1);
        let fx = r.tap_rect(CH0[1]);
        assert_eq!(fx.reveal, Some(1));
        assert_eq!(story_state(&r.m), (0, 0b10, Some(1)));
        assert_eq!(fx.leds, (255, 255), "both LEDs flash on a hit");
        let later = r.tick(FLASH_MS + 1);
        assert_ne!(later.leds, (255, 255), "the flash ends");
    }

    #[test]
    fn a_miss_only_makes_a_ripple() {
        let mut r = Rig::new();
        r.tap(1, 1);
        let fx = r.tap(10, 200);
        assert_eq!(fx.ripple, Some(Point::new(10, 200)));
        assert_eq!(fx.reveal, None);
        assert!(!fx.redraw);
        assert_eq!(story_state(&r.m), (0, 0, None));
    }

    #[test]
    fn next_is_locked_until_every_object_is_found() {
        let mut r = Rig::new();
        r.tap(1, 1);
        let fx = r.tap_next();
        assert_eq!(
            fx.nudge,
            Some(0),
            "locked Next points at the first unfound object"
        );
        assert_eq!(story_state(&r.m).0, 0);

        r.tap_rect(CH0[0]);
        assert_eq!(r.tap_next().nudge, Some(1));
        r.tap_rect(CH0[1]);
        let fx = r.tap_next();
        assert!(fx.scene_changed);
        assert_eq!(story_state(&r.m), (1, 0, None));
    }

    #[test]
    fn chapter_complete_fires_exactly_once() {
        let mut r = Rig::new();
        r.tap(1, 1);
        assert!(!r.tap_rect(CH0[0]).chapter_complete);
        assert!(r.tap_rect(CH0[1]).chapter_complete);
        assert!(
            !r.tap_rect(CH0[1]).chapter_complete,
            "re-tapping does not repeat it"
        );
    }

    #[test]
    fn finishing_the_last_chapter_leads_to_the_question() {
        let mut r = Rig::new();
        r.play_to_question();
        assert_eq!(r.m.no_rect, INITIAL_NO);
        assert_eq!(r.m.dodges, 0);
    }

    #[test]
    fn touching_near_no_makes_it_jump_clear_of_yes() {
        let mut r = Rig::new();
        r.play_to_question();
        let before = r.m.no_rect;
        let fx = r.tap_rect(before);
        assert!(fx.dodged && fx.redraw);
        assert_eq!(r.m.dodges, 1);
        assert_ne!(r.m.no_rect, before);
        assert!(!r.m.no_rect.overlaps(&r.m.yes_rect(), 24));
        assert!(r.m.no_rect.x >= 10 && r.m.no_rect.x + r.m.no_rect.w <= 470);
    }

    #[test]
    fn a_touch_far_from_no_does_nothing() {
        let mut r = Rig::new();
        r.play_to_question();
        let fx = r.tap(20, 20);
        assert!(!fx.dodged && !fx.scene_changed);
        assert_eq!(r.m.dodges, 0);
    }

    #[test]
    fn no_gives_up_after_five_dodges_and_becomes_okay_yes() {
        let mut r = Rig::new();
        r.play_to_question();
        for _ in 0..MAX_DODGES {
            assert!(!r.m.no_gave_up());
            let target = r.m.no_rect;
            assert!(r.tap_rect(target).dodged);
        }
        assert!(r.m.no_gave_up());
        assert_eq!(r.m.no_rect.w, GIVE_UP_W);
        assert!(
            r.m.no_rect.x + r.m.no_rect.w <= 470,
            "stays on screen after widening"
        );

        let still = r.m.no_rect;
        let fx = r.tap_rect(still);
        assert!(!fx.dodged);
        assert_eq!(r.m.scene, Scene::Celebration);
    }

    #[test]
    fn yes_always_works_even_after_dodges() {
        let mut r = Rig::new();
        r.play_to_question();
        let target = r.m.no_rect;
        r.tap_rect(target);
        let yes = r.m.yes_rect();
        r.tap_rect(yes);
        assert_eq!(r.m.scene, Scene::Celebration);
    }

    #[test]
    fn start_again_on_the_celebration_goes_back_to_the_intro() {
        let mut r = Rig::new();
        r.play_to_question();
        let yes = r.m.yes_rect();
        r.tap_rect(yes);
        let fx = r.tap_rect(RESTART_BUTTON);
        assert_eq!(r.m.scene, Scene::Intro);
        assert!(fx.scene_changed && fx.redraw);
    }

    #[test]
    fn restart_goes_back_to_the_intro_from_anywhere() {
        let mut r = Rig::new();
        r.tap(1, 1);
        r.tap_rect(CH0[0]);
        let fx = update(&mut r.m, &Fake, &mut r.rng, Event::Restart);
        assert_eq!(r.m.scene, Scene::Intro);
        assert!(fx.scene_changed);
        r.play_to_question();
        let target = r.m.no_rect;
        r.tap_rect(target);
        update(&mut r.m, &Fake, &mut r.rng, Event::Restart);
        assert_eq!(r.m.scene, Scene::Intro);
        r.play_to_question();
        assert_eq!(
            (r.m.dodges, r.m.no_rect),
            (0, INITIAL_NO),
            "NO starts over too"
        );
    }

    #[test]
    fn a_touch_on_the_celebration_does_nothing() {
        let mut r = Rig::new();
        r.play_to_question();
        let yes = r.m.yes_rect();
        r.tap_rect(yes);
        let fx = r.tap(100, 100);
        assert!(!fx.redraw && !fx.scene_changed);
        assert_eq!(r.m.scene, Scene::Celebration);
    }
}
