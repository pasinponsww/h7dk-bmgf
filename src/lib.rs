#![no_std]

pub mod common;
pub mod fonts;
pub mod screen;
mod stagecraft;
pub mod task;

pub use common::event::{Effects, Event, TouchEvent};
pub use common::state::{Model, Scene};
pub use common::story::Story;
pub use common::{
    album, calendar, event, frame_stats, gesture, liveness, moment, pacing, save, secret, state,
    story, trace, warmth,
};
pub use geometry::{Point, Rect};
pub use rng::{Rng, XorShift32};
pub use task::{leds, update};
