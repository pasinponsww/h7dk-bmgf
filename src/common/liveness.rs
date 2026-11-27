//! When the watchdog may be fed.
//!
//! The tick task feeds the watchdog, but a running tick task alone does not
//! mean the board is fine: the render task runs at a lower priority and could
//! hang on its own while the tick keeps going. So the tick feeds the watchdog
//! only while the render task keeps finishing frames.

/// The render task must finish a frame at least this often, or the watchdog is
/// no longer fed and restarts the board.
pub const RENDER_STALL_MS: u32 = 1_000;

/// Watches the render task's frame counter.
#[derive(Copy, Clone, Debug)]
pub struct Liveness {
    frames: u32,
    since_ms: u32,
}

impl Liveness {
    /// Starts watching at `now_ms`, with no frame drawn yet.
    pub const fn new(now_ms: u32) -> Self {
        Self {
            frames: 0,
            since_ms: now_ms,
        }
    }

    /// `frames` is how many frames the render task has finished so far. True
    /// while it finished one less than [`RENDER_STALL_MS`] ago.
    pub fn alive(&mut self, frames: u32, now_ms: u32) -> bool {
        if frames != self.frames {
            self.frames = frames;
            self.since_ms = now_ms;
        }
        now_ms.wrapping_sub(self.since_ms) < RENDER_STALL_MS
    }
}

#[cfg(test)]
mod tests {
    use super::{Liveness, RENDER_STALL_MS};

    #[test]
    fn the_first_frame_gets_a_full_grace_period() {
        let mut l = Liveness::new(100);
        assert!(l.alive(0, 100 + RENDER_STALL_MS - 1));
        assert!(!l.alive(0, 100 + RENDER_STALL_MS));
    }

    #[test]
    fn new_frames_keep_it_alive() {
        let mut l = Liveness::new(0);
        for (frame, t) in (1..200u32).map(|f| (f, f * 33)) {
            assert!(l.alive(frame, t), "frame {frame} at {t} ms");
        }
    }

    #[test]
    fn a_stalled_render_task_stops_the_feeding() {
        let mut l = Liveness::new(0);
        assert!(l.alive(5, 500));
        assert!(l.alive(5, 500 + RENDER_STALL_MS - 1));
        assert!(!l.alive(5, 500 + RENDER_STALL_MS));
        assert!(!l.alive(5, 500 + 10 * RENDER_STALL_MS));
    }

    #[test]
    fn the_clock_wrapping_around_does_not_matter() {
        let start = u32::MAX - 100;
        let mut l = Liveness::new(start);
        assert!(l.alive(1, start.wrapping_add(50)));
        assert!(l.alive(2, start.wrapping_add(400)));
        assert!(!l.alive(2, start.wrapping_add(400 + RENDER_STALL_MS)));
    }
}
