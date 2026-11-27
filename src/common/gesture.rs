//! Swipes: a finger that goes down, moves quickly sideways and lifts.
//!
//! The firmware reads the touch panel every tick and feeds where the finger
//! is (or that there is none) to [`Gesture::update`], which reports a swipe
//! when the finger lifts. A touch going down is still a tap as well; if that
//! tap already did something big (changed the scene), the firmware calls
//! [`Gesture::cancel`] so the same touch does not also turn the page.

use geometry::Point;

/// The finger must travel at least this far sideways, in pixels.
pub const SWIPE_MIN_PX: i16 = 70;
/// ... and take no longer than this, in milliseconds.
pub const SWIPE_MAX_MS: u32 = 800;

/// Which way a swipe went.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Swipe {
    /// Right to left: the next page.
    Left,
    /// Left to right: the page before.
    Right,
}

/// Follows one finger from going down to lifting.
#[derive(Copy, Clone, Debug)]
pub struct Gesture {
    /// Where and when the finger went down, while it is down.
    start: Option<(Point, u32)>,
    /// Where it was last seen.
    last: Point,
    /// This touch must not count as a swipe.
    cancelled: bool,
}

impl Default for Gesture {
    fn default() -> Self {
        Self::new()
    }
}

impl Gesture {
    /// No finger down.
    pub const fn new() -> Self {
        Self {
            start: None,
            last: Point::new(0, 0),
            cancelled: false,
        }
    }

    /// The finger is at `finger` (or lifted, `None`) at `now_ms`. Returns a
    /// swipe when the finger lifts after one.
    pub fn update(&mut self, finger: Option<Point>, now_ms: u32) -> Option<Swipe> {
        match (finger, self.start) {
            (Some(p), None) => {
                self.start = Some((p, now_ms));
                self.last = p;
                self.cancelled = false;
                None
            }
            (Some(p), Some(_)) => {
                self.last = p;
                None
            }
            (None, Some((from, at))) => {
                self.start = None;
                if self.cancelled || now_ms.wrapping_sub(at) > SWIPE_MAX_MS {
                    return None;
                }
                let dx = i32::from(self.last.x) - i32::from(from.x);
                let dy = i32::from(self.last.y) - i32::from(from.y);
                // Far enough, and clearly more sideways than up or down.
                if dx.abs() < i32::from(SWIPE_MIN_PX) || dx.abs() < 2 * dy.abs() {
                    return None;
                }
                Some(if dx < 0 { Swipe::Left } else { Swipe::Right })
            }
            (None, None) => None,
        }
    }

    /// The current touch will not count as a swipe, whatever it does next.
    pub fn cancel(&mut self) {
        self.cancelled = true;
    }
}

#[cfg(test)]
mod tests {
    use super::{Gesture, Swipe, SWIPE_MAX_MS};
    use geometry::Point;

    /// Feeds a finger moving from `from` to `to` over `ms`, then lifting.
    fn swipe(from: (i16, i16), to: (i16, i16), ms: u32) -> Option<Swipe> {
        let mut g = Gesture::new();
        let steps = 8i16;
        for i in 0..=steps {
            let x = from.0 + (to.0 - from.0) * i / steps;
            let y = from.1 + (to.1 - from.1) * i / steps;
            let t = ms * u32::try_from(i).unwrap() / u32::try_from(steps).unwrap();
            assert_eq!(g.update(Some(Point::new(x, y)), 1_000 + t), None);
        }
        g.update(None, 1_000 + ms + 16)
    }

    #[test]
    fn a_quick_sideways_move_is_a_swipe() {
        assert_eq!(swipe((400, 130), (200, 140), 250), Some(Swipe::Left));
        assert_eq!(swipe((100, 130), (300, 120), 250), Some(Swipe::Right));
    }

    #[test]
    fn short_slow_or_up_and_down_moves_are_not() {
        assert_eq!(swipe((300, 130), (250, 130), 200), None, "too short");
        assert_eq!(
            swipe((400, 130), (200, 130), SWIPE_MAX_MS + 50),
            None,
            "too slow"
        );
        assert_eq!(swipe((300, 40), (220, 240), 200), None, "mostly down");
        assert_eq!(swipe((300, 130), (300, 130), 100), None, "a tap");
    }

    #[test]
    fn a_cancelled_touch_is_not_a_swipe_but_the_next_one_can_be() {
        let mut g = Gesture::new();
        g.update(Some(Point::new(400, 100)), 0);
        g.cancel();
        g.update(Some(Point::new(200, 100)), 100);
        assert_eq!(g.update(None, 200), None);
        g.update(Some(Point::new(400, 100)), 300);
        g.update(Some(Point::new(200, 100)), 400);
        assert_eq!(g.update(None, 500), Some(Swipe::Left));
    }

    #[test]
    fn nothing_happens_without_a_finger() {
        let mut g = Gesture::new();
        assert_eq!(g.update(None, 0), None);
        assert_eq!(g.update(None, 16), None);
    }
}
