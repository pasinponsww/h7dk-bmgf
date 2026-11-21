pub mod leds;
mod tick;
mod touch;

use rng::Rng;

use crate::event::{Effects, Event, TouchEvent};
use crate::state::Model;
use crate::story::Story;

/// Feeds one event to the state machine and returns what to do about it.
pub fn update<S: Story, R: Rng>(m: &mut Model, story: &S, rng: &mut R, ev: Event) -> Effects {
    let mut fx = Effects::default();
    match ev {
        Event::Tick { now_ms } => {
            m.now_ms = now_ms;
            tick::on_tick(m, story, &mut fx);
        }
        Event::Touch(TouchEvent::Down(p)) => touch::on_down(m, story, rng, p, &mut fx),
        Event::Touch(TouchEvent::Move(_) | TouchEvent::Up(_)) => {}
        Event::Restart => touch::restart(m, &mut fx),
    }
    fx.leds = leds::output(m);
    fx
}
