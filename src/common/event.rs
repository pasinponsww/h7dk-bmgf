//! Inputs to the state machine and the effects it returns.

use geometry::Point;

/// Something the hardware layer reports.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Event {
    /// A clock tick. Send one every 10 to 16 ms with the current time.
    Tick {
        /// Milliseconds since boot.
        now_ms: u32,
    },
    /// A touch on the screen.
    Touch(TouchEvent),
    /// Go back to the intro from wherever the story is (a long press on the
    /// blue button).
    Restart,
}

/// A touch reported by the touch controller.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum TouchEvent {
    /// Finger went down at this point.
    Down(Point),
    /// Finger moved to this point.
    Move(Point),
    /// Finger lifted at this point.
    Up(Point),
}

/// What the hardware and UI layers should do after an event.
///
/// `leds` is always filled in. The other fields are one-shot cues and are
/// empty or false most of the time.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Default)]
pub struct Effects {
    /// The screen content changed and should be drawn again.
    pub redraw: bool,
    /// Desired LED duty, `(green, red)`, from 0 (off) to 255 (always on).
    pub leds: (u8, u8),
    /// Draw a touch ripple here.
    pub ripple: Option<Point>,
    /// A hotspot was tapped: show its line and play its pop animation.
    pub reveal: Option<u8>,
    /// Pulse this hotspot as a hint.
    pub nudge: Option<u8>,
    /// Every hotspot in the chapter was just found.
    pub chapter_complete: bool,
    /// The scene changed (use it to start a crossfade).
    pub scene_changed: bool,
    /// The NO button just jumped to a new place.
    pub dodged: bool,
    /// The dog was just petted.
    pub petted: bool,
}
