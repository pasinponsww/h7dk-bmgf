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

/// [`Fake`] with the trace-a-heart game before the question.
pub struct FakeTrace;

impl Story for FakeTrace {
    fn chapter_count(&self) -> usize {
        Fake.chapter_count()
    }
    fn hotspot_count(&self, chapter: usize) -> usize {
        Fake.hotspot_count(chapter)
    }
    fn hotspot_rect(&self, chapter: usize, index: usize) -> Rect {
        Fake.hotspot_rect(chapter, index)
    }
    fn trace_heart(&self) -> bool {
        true
    }
}

/// A model, a random source and shortcuts for sending events.
pub struct Rig {
    pub m: Model,
    pub rng: XorShift32,
    /// Use [`FakeTrace`] instead of [`Fake`].
    pub trace: bool,
}

impl Rig {
    pub fn new() -> Self {
        Self {
            m: Model::new(0),
            rng: XorShift32::new(7),
            trace: false,
        }
    }

    /// A rig whose story has the trace-a-heart game.
    pub fn with_trace() -> Self {
        Self {
            trace: true,
            ..Self::new()
        }
    }

    pub fn send(&mut self, ev: Event) -> Effects {
        if self.trace {
            update(&mut self.m, &FakeTrace, &mut self.rng, ev)
        } else {
            update(&mut self.m, &Fake, &mut self.rng, ev)
        }
    }

    pub fn tick(&mut self, t: u32) -> Effects {
        self.send(Event::Tick { now_ms: t })
    }

    pub fn tap(&mut self, x: i16, y: i16) -> Effects {
        self.send(Event::Touch(TouchEvent::Down(Point::new(x, y))))
    }

    pub fn drag(&mut self, x: i16, y: i16) -> Effects {
        self.send(Event::Touch(TouchEvent::Move(Point::new(x, y))))
    }

    pub fn tap_rect(&mut self, r: Rect) -> Effects {
        self.tap(r.x + r.w / 2, r.y + r.h / 2)
    }

    pub fn tap_next(&mut self) -> Effects {
        self.tap_rect(NEXT_BUTTON)
    }

    /// Intro, then both chapters fully. Ends on the question screen, or on
    /// the trace game for a rig made [`Rig::with_trace`].
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
        if self.trace {
            assert_eq!(self.m.scene, Scene::Trace { hit: 0 });
        } else {
            assert_eq!(self.m.scene, Scene::Question);
        }
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
