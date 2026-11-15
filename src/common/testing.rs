//! Helpers shared by the unit tests of the logic.
#![allow(dead_code)]

use crate::state::NEXT_BUTTON;
use crate::{update, Effects, Event, Model, Point, Rect, Scene, Story, TouchEvent, XorShift32};

/// A tiny two-chapter story: 2 objects, then 3 objects.
pub struct Fake;

pub const CH0: [Rect; 2] = [Rect::new(286, 52, 64, 64), Rect::new(366, 128, 64, 64)];
pub const CH1: [Rect; 3] = [
    Rect::new(268, 36, 64, 64),
    Rect::new(372, 66, 64, 64),
    Rect::new(304, 134, 64, 64),
];

impl Story for Fake {
    fn chapter_count(&self) -> usize {
        2
    }
    fn hotspot_count(&self, chapter: usize) -> usize {
        if chapter == 0 {
            CH0.len()
        } else {
            CH1.len()
        }
    }
    fn hotspot_rect(&self, chapter: usize, index: usize) -> Rect {
        if chapter == 0 {
            CH0[index]
        } else {
            CH1[index]
        }
    }
}

/// A model, a random source and shortcuts for sending events.
pub struct Rig {
    pub m: Model,
    pub rng: XorShift32,
}

impl Rig {
    pub fn new() -> Self {
        Self {
            m: Model::new(0),
            rng: XorShift32::new(7),
        }
    }

    pub fn tick(&mut self, t: u32) -> Effects {
        update(&mut self.m, &Fake, &mut self.rng, Event::Tick { now_ms: t })
    }

    pub fn tap(&mut self, x: i16, y: i16) -> Effects {
        update(
            &mut self.m,
            &Fake,
            &mut self.rng,
            Event::Touch(TouchEvent::Down(Point::new(x, y))),
        )
    }

    pub fn tap_rect(&mut self, r: Rect) -> Effects {
        self.tap(r.x + r.w / 2, r.y + r.h / 2)
    }

    pub fn tap_next(&mut self) -> Effects {
        self.tap_rect(NEXT_BUTTON)
    }

    /// Intro, then both chapters fully, ending on the question screen.
    pub fn play_to_question(&mut self) {
        self.tap(10, 10);
        for r in CH0 {
            self.tap_rect(r);
        }
        self.tap_next();
        for r in CH1 {
            self.tap_rect(r);
        }
        self.tap_next();
        assert_eq!(self.m.scene, Scene::Question);
    }
}

/// `(chapter, found, shown)` of a story scene. Panics on any other scene.
pub fn story_state(m: &Model) -> (u8, u8, Option<u8>) {
    match m.scene {
        Scene::Story {
            chapter,
            found,
            shown,
        } => (chapter, found, shown),
        other => panic!("expected a story scene, got {other:?}"),
    }
}
