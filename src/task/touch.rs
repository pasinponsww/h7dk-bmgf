//! The touch task: a finger went down. What it does depends on the screen.

use geometry::place::{farthest_free_spot, Placement};
use geometry::Point;
use rng::Rng;

use crate::event::Effects;
use crate::gesture::Swipe;
use crate::state::{
    Model, Scene, DODGE_RADIUS, FLASH_MS, GIVE_UP_W, JUMP_TRIES, MAX_DODGES, NEXT_BUTTON, NO_AREA,
    PHOTO_CARD, RESTART_BUTTON, SCREEN_W, YES_PAD,
};
use crate::story::{full_mask, Story};
use crate::trace;

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
        Scene::Intro if m.dog_rect().contains(p) => pet(m, fx),
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
        Scene::Trace { hit } => {
            if m.skip_ok && m.traced_ms.is_none() && NEXT_BUTTON.contains(p) {
                // Given up on drawing: straight on to the question.
                m.go(Scene::Question);
                fx.scene_changed = true;
                fx.redraw = true;
            } else if m.dog_rect().contains(p) {
                pet(m, fx);
            } else {
                // A new stroke: nothing joins it to where the last one ended.
                m.pen = None;
                trace_to(m, hit, p, fx);
            }
        }
        Scene::Question => question_touch(m, rng, p, fx),
        Scene::Celebration => {
            if RESTART_BUTTON.contains(p) {
                restart(m, fx);
            } else if m.dog_rect().contains(p) {
                pet(m, fx);
            }
        }
    }
}

/// The finger lifted: the next touch starts a new stroke.
pub(crate) fn on_up(m: &mut Model) {
    m.pen = None;
}

/// The finger moved while down. Only tracing the heart cares.
pub(crate) fn on_move(m: &mut Model, p: Point, fx: &mut Effects) {
    if let Scene::Trace { hit } = m.scene {
        m.last_touch_ms = m.now_ms;
        trace_to(m, hit, p, fx);
    }
}

/// Traces the dots the finger passed on its way to `p`, and finishes the
/// heart once enough are traced.
fn trace_to(m: &mut Model, hit: u32, p: Point, fx: &mut Effects) {
    if m.traced_ms.is_some() {
        return;
    }
    let now = trace::mark(hit, m.pen, p);
    m.pen = Some(p);
    if now == hit {
        return;
    }
    m.scene = Scene::Trace { hit: now };
    fx.redraw = true;
    if trace::done(now) {
        m.traced_ms = Some(m.now_ms);
        m.flash_until_ms = m.now_ms.wrapping_add(FLASH_MS);
        fx.chapter_complete = true;
    }
}

/// The dog was petted: it falls in love for a moment and both LEDs flash.
fn pet(m: &mut Model, fx: &mut Effects) {
    m.petted_ms = Some(m.now_ms);
    m.flash_until_ms = m.now_ms.wrapping_add(FLASH_MS);
    fx.petted = true;
    fx.redraw = true;
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

    // A tap on the polaroid puts it aside, so the objects are back.
    if m.photo_ms.is_some() && PHOTO_CARD.contains(p) {
        m.photo_ms = None;
        fx.redraw = true;
        return;
    }

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
            // The last object found: out comes the chapter's photo.
            if fx.chapter_complete && story.has_photo(ch) {
                m.photo_ms = Some(m.now_ms);
            }
            return;
        }
    }

    // Did she tap "Next"?
    if NEXT_BUTTON.contains(p) {
        turn_page(m, story, ch, found, fx);
        return;
    }

    if m.dog_rect().contains(p) {
        pet(m, fx);
    }
    // Anything else is a miss: the ripple is the only feedback.
}

/// "Next" on chapter `ch`: on to the next chapter or the question once every
/// object is found (or the page has waited long enough), otherwise a hint.
fn turn_page<S: Story>(m: &mut Model, story: &S, ch: usize, found: u8, fx: &mut Effects) {
    let n = story.hotspot_count(ch);
    if found != full_mask(n) && !m.skip_ok {
        fx.nudge = first_unfound(found, n);
        return;
    }
    let next = ch + 1;
    if next >= story.chapter_count() {
        m.go(if story.trace_heart() {
            Scene::Trace { hit: 0 }
        } else {
            Scene::Question
        });
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

/// A swipe on a story page turns it: to the left is "Next", to the right goes
/// back a chapter, with its objects already found so she can read them again.
/// Other scenes ignore swipes.
pub(crate) fn on_swipe<S: Story>(m: &mut Model, story: &S, swipe: Swipe, fx: &mut Effects) {
    m.last_touch_ms = m.now_ms;
    let Scene::Story { chapter, found, .. } = m.scene else {
        return;
    };
    let ch = usize::from(chapter);
    match swipe {
        Swipe::Left => turn_page(m, story, ch, found, fx),
        Swipe::Right => {
            let Some(back) = ch.checked_sub(1) else {
                return;
            };
            m.go(Scene::Story {
                chapter: u8::try_from(back).unwrap_or(0),
                found: full_mask(story.hotspot_count(back)),
                shown: None,
            });
            // Back on a finished page: its photo is out again.
            if story.has_photo(back) {
                m.photo_ms = Some(m.now_ms);
            }
            fx.scene_changed = true;
            fx.redraw = true;
        }
    }
}

fn question_touch<R: Rng>(m: &mut Model, rng: &mut R, p: Point, fx: &mut Effects) {
    // YES always wins, wherever and whenever.
    if m.yes_rect().contains(p) {
        celebrate(m, fx);
        return;
    }
    // Petting the dog in the corner is not a try at NO.
    if m.dog_rect().contains(p) {
        pet(m, fx);
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
    use crate::gesture::Swipe;
    use crate::state::{
        dog_rect, Scene, FLASH_MS, GIVE_UP_W, INITIAL_NO, MAX_DODGES, PET_MS, RESTART_BUTTON,
    };
    use crate::state::{NUDGE_AFTER_MS, TRACE_ADMIRE_MS, UNLOCK_AFTER_MS};
    use crate::trace::{DOTS, DOT_AT};
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
        let fx = r.tap(330, 100);
        assert!(!fx.redraw && !fx.scene_changed);
        assert_eq!(r.m.scene, Scene::Celebration);
    }

    fn swipe(r: &mut Rig, s: Swipe) -> crate::Effects {
        update(&mut r.m, &Fake, &mut r.rng, Event::Swipe(s))
    }

    #[test]
    fn swiping_left_turns_the_page_like_next() {
        let mut r = Rig::new();
        r.tap(1, 1);
        let fx = swipe(&mut r, Swipe::Left);
        assert_eq!(fx.nudge, Some(0), "locked: a hint instead");
        assert_eq!(story_state(&r.m).0, 0);
        for c in CH0 {
            r.tap_rect(c);
        }
        let fx = swipe(&mut r, Swipe::Left);
        assert!(fx.scene_changed);
        assert_eq!(story_state(&r.m), (1, 0, None));
    }

    #[test]
    fn swiping_right_goes_back_a_chapter_with_everything_found() {
        let mut r = Rig::new();
        r.tap(1, 1);
        assert!(
            !swipe(&mut r, Swipe::Right).scene_changed,
            "nothing before chapter 1"
        );
        for c in CH0 {
            r.tap_rect(c);
        }
        r.tap_next();
        let fx = swipe(&mut r, Swipe::Right);
        assert!(fx.scene_changed);
        assert_eq!(story_state(&r.m), (0, 0b11, None));
        assert!(r.m.next_unlocked(&Fake), "Next works again at once");
    }

    #[test]
    fn swipes_do_nothing_outside_the_story() {
        let mut r = Rig::new();
        assert!(!swipe(&mut r, Swipe::Left).scene_changed);
        assert_eq!(r.m.scene, Scene::Intro);
        r.play_to_question();
        assert!(!swipe(&mut r, Swipe::Right).scene_changed);
        assert_eq!(r.m.scene, Scene::Question);
    }

    /// Traces the heart: a tap on the first dot, then a drag round the rest.
    fn trace_heart(r: &mut Rig) {
        let (x, y) = DOT_AT[0];
        r.tap(x, y);
        for i in 1..=DOTS {
            let (x, y) = DOT_AT[i % DOTS];
            r.drag(x, y);
        }
    }

    #[test]
    fn after_the_last_chapter_she_traces_a_heart_before_the_question() {
        let mut r = Rig::with_trace();
        r.play_to_question();
        trace_heart(&mut r);
        let Scene::Trace { hit } = r.m.scene else {
            panic!("still tracing");
        };
        assert!(crate::trace::done(hit), "{hit:#b}");
        assert_eq!(r.m.traced_for(r.m.now_ms), Some(0));
        r.tick(TRACE_ADMIRE_MS - 1);
        assert!(
            matches!(r.m.scene, Scene::Trace { .. }),
            "admiring the heart"
        );
        let fx = r.tick(TRACE_ADMIRE_MS);
        assert!(fx.scene_changed);
        assert_eq!(r.m.scene, Scene::Question);
    }

    #[test]
    fn lifting_the_finger_starts_a_new_stroke() {
        let mut r = Rig::with_trace();
        r.play_to_question();
        let (a, b) = (DOT_AT[0], DOT_AT[12]);
        r.tap(a.0, a.1);
        r.tap(b.0, b.1);
        assert_eq!(
            r.m.scene,
            Scene::Trace { hit: 1 | 1 << 12 },
            "no line across the middle"
        );
    }

    #[test]
    fn stuck_on_the_heart_gets_a_hint_then_a_way_on() {
        let mut r = Rig::with_trace();
        r.play_to_question();
        assert_eq!(r.tick(NUDGE_AFTER_MS + 100).nudge, Some(0));
        r.tap_next();
        assert!(
            matches!(r.m.scene, Scene::Trace { .. }),
            "Next is not there yet"
        );
        // The tap on Next counted as a touch, so the wait starts from there.
        r.tick(NUDGE_AFTER_MS + 100 + UNLOCK_AFTER_MS + 100);
        assert!(r.m.next_unlocked(&Fake));
        let fx = r.tap_next();
        assert!(fx.scene_changed);
        assert_eq!(r.m.scene, Scene::Question);
    }

    #[test]
    fn a_drag_does_nothing_outside_the_heart_game() {
        let mut r = Rig::new();
        r.tap(1, 1);
        let before = r.m.scene;
        let fx = r.drag(CH0[0].x + 4, CH0[0].y + 4);
        assert!(!fx.redraw);
        assert_eq!(r.m.scene, before, "dragging over an object does not tap it");
    }

    #[test]
    fn petting_the_dog_on_the_intro_does_not_open_the_story() {
        let mut r = Rig::new();
        let fx = r.tap_rect(dog_rect(Scene::Intro));
        assert!(fx.petted && !fx.scene_changed);
        assert_eq!(r.m.scene, Scene::Intro);
        assert_eq!(fx.leds, (255, 255), "both LEDs flash");
        assert_eq!(r.m.petted_for(r.m.now_ms), Some(0));
    }

    #[test]
    fn petting_the_dog_on_a_story_page_keeps_the_page() {
        let mut r = Rig::new();
        r.tap(1, 1);
        r.tap_rect(CH0[0]);
        let fx = r.tap_rect(dog_rect(r.m.scene));
        assert!(fx.petted && fx.redraw);
        assert_eq!(
            story_state(&r.m),
            (0, 0b01, Some(0)),
            "the page is unchanged"
        );
    }

    #[test]
    fn the_dog_in_the_corner_is_not_a_try_at_no() {
        let mut r = Rig::new();
        r.play_to_question();
        let no = r.m.no_rect;
        let fx = r.tap_rect(dog_rect(Scene::Question));
        assert!(fx.petted && !fx.dodged);
        assert_eq!((r.m.dodges, r.m.no_rect), (0, no));
    }

    #[test]
    fn the_dog_can_be_petted_on_the_celebration_too() {
        let mut r = Rig::new();
        r.play_to_question();
        let yes = r.m.yes_rect();
        r.tap_rect(yes);
        let fx = r.tap_rect(dog_rect(Scene::Celebration));
        assert!(fx.petted);
        assert_eq!(r.m.scene, Scene::Celebration);
    }

    #[test]
    fn the_love_wears_off_and_a_new_scene_forgets_it() {
        let mut r = Rig::new();
        r.tap_rect(dog_rect(Scene::Intro));
        r.tick(PET_MS - 1);
        assert_eq!(r.m.petted_for(r.m.now_ms), Some(PET_MS - 1));
        r.tick(PET_MS);
        assert_eq!(r.m.petted_for(r.m.now_ms), None, "worn off");
        r.tap_rect(dog_rect(Scene::Intro));
        r.tap(300, 30);
        assert_eq!(r.m.petted_for(r.m.now_ms), None, "the story starts fresh");
    }
}
