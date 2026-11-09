
#![no_std]
#![warn(missing_docs)]

pub mod common;
pub mod task;

pub use common::event::{Effects, Event, TouchEvent};
pub use common::state::{Model, Scene};
pub use common::story::Story;
pub use common::{event, state, story};
pub use geometry::{Point, Rect};
pub use rng::{Rng, XorShift32};
pub use task::update;
