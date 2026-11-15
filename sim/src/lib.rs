//! Browser simulator.
//! To see if the application logic working around the story is correct or not

use std::cell::RefCell;

use app::state::NEXT_BUTTON;
use app::story::{Content, CHAPTERS};
use app::{update, Effects, Event, Model, Point, Scene, Story, TouchEvent, XorShift32};

struct State {
    model: Model,
    rng: XorShift32,
    fx: Effects,
}

impl State {
    fn new(seed: u32, now_ms: u32) -> Self {
        Self {
            model: Model::new(now_ms),
            rng: XorShift32::new(seed),
            fx: Effects::default(),
        }
    }

    fn send(&mut self, ev: Event) {
        self.fx = update(&mut self.model, &Content, &mut self.rng, ev);
    }

    fn tap(&mut self, x: i16, y: i16) {
        self.send(Event::Touch(TouchEvent::Down(Point::new(x, y))));
    }
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State::new(1, 0));
}

fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    STATE.with(|s| f(&mut s.borrow_mut()))
}

fn to_u32(v: i32) -> u32 {
    u32::try_from(v).unwrap_or(0)
}

fn opt(v: Option<u8>) -> i32 {
    v.map_or(-1, i32::from)
}

// Control

/// Starts over on the intro screen.
#[no_mangle]
pub extern "C" fn sim_init(seed: u32, now_ms: u32) {
    with(|s| *s = State::new(seed, now_ms));
}

/// Advances the clock. Call about 60 times a second.
#[no_mangle]
pub extern "C" fn sim_tick(now_ms: u32) {
    with(|s| s.send(Event::Tick { now_ms }));
}

/// A finger touching the screen at (x, y), in 480 x 272 pixels.
#[no_mangle]
pub extern "C" fn sim_touch(x: i32, y: i32) {
    let x = i16::try_from(x).unwrap_or(0);
    let y = i16::try_from(y).unwrap_or(0);
    with(|s| s.tap(x, y));
}

/// Debug shortcut: plays the real story to reach a scene.
/// `scene` 0 intro, 1 story (at `chapter`), 2 question, 3 celebration.
#[no_mangle]
pub extern "C" fn sim_skip_to(scene: u32, chapter: u32, now_ms: u32) {
    with(|s| {
        *s = State::new(1, now_ms);
        s.send(Event::Tick { now_ms });
        if scene == 0 {
            s.fx = Effects::default();
            return;
        }
        s.tap(1, 1); // open the story
        let stop = if scene == 1 {
            usize::try_from(chapter)
                .unwrap_or(0)
                .min(CHAPTERS.len() - 1)
        } else {
            CHAPTERS.len()
        };
        for ch in 0..stop {
            for i in 0..Content.hotspot_count(ch) {
                let r = Content.hotspot_rect(ch, i);
                s.tap(r.x + 2, r.y + 2);
            }
            s.tap(NEXT_BUTTON.x + 2, NEXT_BUTTON.y + 2);
        }
        if scene >= 3 {
            let y = s.model.yes_rect();
            s.tap(y.x + 2, y.y + 2);
        }
        s.fx = Effects::default();
    });
}

// Expose state to the simulator

/// Bit flags: 1 redraw, 2 scene changed, 4 chapter complete, 8 NO dodged,
/// 16 ripple, 32 object revealed, 64 nudge.
#[no_mangle]
pub extern "C" fn fx_flags() -> u32 {
    with(|s| {
        let f = &s.fx;
        u32::from(f.redraw)
            | u32::from(f.scene_changed) << 1
            | u32::from(f.chapter_complete) << 2
            | u32::from(f.dodged) << 3
            | u32::from(f.ripple.is_some()) << 4
            | u32::from(f.reveal.is_some()) << 5
            | u32::from(f.nudge.is_some()) << 6
    })
}

/// LED state: bit 0 green, bit 1 red.
#[no_mangle]
pub extern "C" fn fx_led() -> u32 {
    with(|s| u32::from(s.fx.leds.0) | u32::from(s.fx.leds.1) << 1)
}

/// Ripple x, or -1.
#[no_mangle]
pub extern "C" fn fx_ripple_x() -> i32 {
    with(|s| s.fx.ripple.map_or(-1, |p| i32::from(p.x)))
}

/// Ripple y, or -1.
#[no_mangle]
pub extern "C" fn fx_ripple_y() -> i32 {
    with(|s| s.fx.ripple.map_or(-1, |p| i32::from(p.y)))
}

/// Index of the object just tapped, or -1.
#[no_mangle]
pub extern "C" fn fx_reveal() -> i32 {
    with(|s| opt(s.fx.reveal))
}

/// Index of the object to pulse as a hint, or -1.
#[no_mangle]
pub extern "C" fn fx_nudge() -> i32 {
    with(|s| opt(s.fx.nudge))
}

// ---------- model state ----------

/// 0 intro, 1 story, 2 question, 3 celebration.
#[no_mangle]
pub extern "C" fn scene() -> u32 {
    with(|s| match s.model.scene {
        Scene::Intro => 0,
        Scene::Story { .. } => 1,
        Scene::Question => 2,
        Scene::Celebration => 3,
    })
}

/// Current chapter (0 when not on a story page).
#[no_mangle]
pub extern "C" fn chapter() -> u32 {
    with(|s| match s.model.scene {
        Scene::Story { chapter, .. } => u32::from(chapter),
        _ => 0,
    })
}

/// Bitmask of objects found on this page.
#[no_mangle]
pub extern "C" fn found() -> u32 {
    with(|s| match s.model.scene {
        Scene::Story { found, .. } => u32::from(found),
        _ => 0,
    })
}

/// Object whose line is shown, or -1.
#[no_mangle]
pub extern "C" fn shown() -> i32 {
    with(|s| match s.model.scene {
        Scene::Story { shown, .. } => opt(shown),
        _ => -1,
    })
}

/// 1 if the Next button works right now.
#[no_mangle]
pub extern "C" fn next_unlocked() -> u32 {
    with(|s| u32::from(s.model.next_unlocked(&Content)))
}

/// 1 if Next was unlocked by idling rather than by finding everything.
#[no_mangle]
pub extern "C" fn skip_ok() -> u32 {
    with(|s| u32::from(s.model.skip_ok))
}

/// How many times NO has jumped.
#[no_mangle]
pub extern "C" fn dodges() -> u32 {
    with(|s| u32::from(s.model.dodges))
}

/// 1 once NO has given up.
#[no_mangle]
pub extern "C" fn no_gave_up() -> u32 {
    with(|s| u32::from(s.model.no_gave_up()))
}

/// NO button rectangle: part 0 x, 1 y, 2 w, 3 h.
#[no_mangle]
pub extern "C" fn no_rect(part: u32) -> i32 {
    with(|s| rect_part(s.model.no_rect, part))
}

/// YES button rectangle: part 0 x, 1 y, 2 w, 3 h.
#[no_mangle]
pub extern "C" fn yes_rect(part: u32) -> i32 {
    with(|s| rect_part(s.model.yes_rect(), part))
}

/// Next button rectangle: part 0 x, 1 y, 2 w, 3 h.
#[no_mangle]
pub extern "C" fn next_rect(part: u32) -> i32 {
    rect_part(NEXT_BUTTON, part)
}

fn rect_part(r: app::Rect, part: u32) -> i32 {
    i32::from(match part {
        0 => r.x,
        1 => r.y,
        2 => r.w,
        _ => r.h,
    })
}

// Story content

/// Number of chapters.
#[no_mangle]
pub extern "C" fn chapter_count() -> u32 {
    u32::try_from(CHAPTERS.len()).unwrap_or(0)
}

/// Number of objects in a chapter.
#[no_mangle]
pub extern "C" fn hotspot_count(ch: u32) -> u32 {
    u32::try_from(Content.hotspot_count(ch as usize)).unwrap_or(0)
}

/// Object rectangle: part 0 x, 1 y, 2 w, 3 h.
#[no_mangle]
pub extern "C" fn hotspot_rect(ch: u32, i: u32, part: u32) -> i32 {
    rect_part(Content.hotspot_rect(ch as usize, i as usize), part)
}

fn text(ch: u32, i: i32, which: u32) -> &'static str {
    let Some(c) = CHAPTERS.get(ch as usize) else {
        return "";
    };
    if i < 0 {
        return c.title;
    }
    match c.hotspots.get(to_u32(i) as usize) {
        Some(h) if which == 1 => h.icon,
        Some(h) => h.line,
        None => "",
    }
}

/// Address of a chapter title in wasm memory.
#[no_mangle]
pub extern "C" fn title_ptr(ch: u32) -> u32 {
    text(ch, -1, 0).as_ptr() as usize as u32
}
/// Length of a chapter title.
#[no_mangle]
pub extern "C" fn title_len(ch: u32) -> u32 {
    text(ch, -1, 0).len() as u32
}
/// Address of an object's icon name.
#[no_mangle]
pub extern "C" fn icon_ptr(ch: u32, i: u32) -> u32 {
    text(ch, i as i32, 1).as_ptr() as usize as u32
}
/// Length of an object's icon name.
#[no_mangle]
pub extern "C" fn icon_len(ch: u32, i: u32) -> u32 {
    text(ch, i as i32, 1).len() as u32
}
/// Address of an object's line of text.
#[no_mangle]
pub extern "C" fn line_ptr(ch: u32, i: u32) -> u32 {
    text(ch, i as i32, 0).as_ptr() as usize as u32
}
/// Length of an object's line of text.
#[no_mangle]
pub extern "C" fn line_len(ch: u32, i: u32) -> u32 {
    text(ch, i as i32, 0).len() as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn skip_to_reaches_each_scene_using_the_real_rules() {
        let last = chapter_count() - 1;
        sim_skip_to(1, last, 0);
        assert_eq!((scene(), chapter()), (1, last));
        sim_skip_to(2, 0, 0);
        assert_eq!(scene(), 2);
        sim_skip_to(3, 0, 0);
        assert_eq!(scene(), 3);
        sim_skip_to(0, 0, 0);
        assert_eq!(scene(), 0);
    }

    #[test]
    fn touch_and_effects_round_trip() {
        sim_init(5, 0);
        sim_touch(240, 136);
        assert_eq!(scene(), 1);
        assert_ne!(fx_flags() & 2, 0, "scene changed flag");
        let x = hotspot_rect(0, 1, 0) + 5;
        let y = hotspot_rect(0, 1, 1) + 5;
        sim_touch(x, y);
        assert_eq!(fx_reveal(), 1);
        assert_eq!(shown(), 1);
        assert_eq!(found(), 0b10);
        assert_eq!(fx_led(), 0b11, "both LEDs flash");
        assert_eq!(next_unlocked(), 0);
    }

    #[test]
    fn strings_are_readable_from_their_pointers() {
        assert_eq!(title_len(0) as usize, CHAPTERS[0].title.len());
        assert_eq!(line_len(0, 0) as usize, CHAPTERS[0].hotspots[0].line.len());
        assert_eq!(icon_len(0, 0) as usize, CHAPTERS[0].hotspots[0].icon.len());
        assert_eq!(title_len(999), 0, "out of range is safe");
    }

    #[test]
    fn the_exported_counts_match_the_story() {
        assert_eq!(chapter_count() as usize, CHAPTERS.len());
    }
}
