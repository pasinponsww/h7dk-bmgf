//! Copying bands to the screen in step with the LCD's refresh.
//!
//! The LCD controller reads the framebuffer top to bottom about 60 times a
//! second. The firmware draws a frame in bands, top to bottom, and copies each
//! band into the framebuffer when it is done. If a band is copied while the
//! controller has not reached it yet in the current refresh, that refresh
//! shows the new band under old ones: a tear, which shows up as a sideways
//! jump through anything that moves.
//!
//! So a frame starts in the blank lines at the top of a refresh, and each band
//! waits until the controller has read past it before it is copied. The
//! refresh then shows the whole old frame and the next one the whole new
//! frame. If drawing falls behind and a new refresh has begun, bands are
//! copied right away: the best that can be done with one framebuffer.

/// The LCD controller's line counter: how many lines one refresh has, where
/// the visible rows start and how long one line takes.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Timing {
    /// The first counter value of the visible rows (after sync and back porch).
    pub active_start: u16,
    /// Counter values in one refresh, blank lines included.
    pub lines: u16,
    /// Nanoseconds per line.
    pub line_ns: u32,
}

impl Timing {
    /// True while the controller is in the blank lines before the first row.
    pub fn in_top_blank(&self, line: u16) -> bool {
        line < self.active_start
    }

    /// Microseconds until the controller reaches the top blank lines of the
    /// next refresh (zero while it is in them).
    pub fn us_until_top(&self, line: u16) -> u32 {
        if self.in_top_blank(line) {
            return 0;
        }
        let left = u32::from(self.lines.saturating_sub(line));
        left * self.line_ns / 1_000
    }

    /// Microseconds one whole refresh takes.
    pub fn refresh_us(&self) -> u32 {
        u32::from(self.lines) * self.line_ns / 1_000
    }
}

/// Follows the controller through one frame and says when a band may be copied.
#[derive(Copy, Clone, Debug)]
pub struct Racer {
    timing: Timing,
    last: u16,
    wrapped: bool,
}

impl Racer {
    /// Starts following at counter value `line`, which should be in the top
    /// blank lines (see [`Timing::in_top_blank`]).
    pub const fn new(timing: Timing, line: u16) -> Self {
        Self {
            timing,
            last: line,
            wrapped: false,
        }
    }

    /// True if the band that ends just above screen row `bottom` may be
    /// copied now that the counter reads `line`: the controller has read past
    /// it in this refresh, or a new refresh has already begun.
    pub fn may_copy(&mut self, line: u16, bottom: u16) -> bool {
        if line < self.last {
            self.wrapped = true;
        }
        self.last = line;
        self.wrapped || line >= self.timing.active_start.saturating_add(bottom)
    }
}

#[cfg(test)]
mod tests {
    use super::{Racer, Timing};

    /// The STM32H735G-DK panel: 10 sync and 2 back porch lines, 272 rows,
    /// 2 front porch lines, 566 pixel clocks per line at 9.63 MHz.
    const DK: Timing = Timing {
        active_start: 12,
        lines: 286,
        line_ns: 58_774,
    };

    #[test]
    fn the_top_blank_is_the_lines_before_the_first_row() {
        assert!(DK.in_top_blank(0));
        assert!(DK.in_top_blank(11));
        assert!(!DK.in_top_blank(12));
        assert!(!DK.in_top_blank(285));
    }

    #[test]
    fn time_until_the_top_counts_the_lines_left() {
        assert_eq!(DK.us_until_top(5), 0);
        assert_eq!(DK.us_until_top(284), 117);
        assert_eq!(DK.us_until_top(12), 16_104);
        assert_eq!(DK.refresh_us(), 16_809, "about 59.5 refreshes a second");
    }

    #[test]
    fn a_band_waits_until_the_controller_has_read_past_it() {
        let mut r = Racer::new(DK, 3);
        assert!(!r.may_copy(10, 16), "still in the blank lines");
        assert!(!r.may_copy(27, 16), "reading row 15, the band's last row");
        assert!(r.may_copy(28, 16), "past row 15");
        assert!(!r.may_copy(40, 48), "the next band is not read yet");
        assert!(r.may_copy(60, 48));
        assert!(r.may_copy(285, 272), "the last band waits for the end");
    }

    #[test]
    fn once_a_new_refresh_begins_everything_is_copied_at_once() {
        let mut r = Racer::new(DK, 3);
        assert!(!r.may_copy(200, 240));
        assert!(r.may_copy(2, 240), "the counter went back to the top");
        assert!(r.may_copy(20, 272), "and stays copy-at-once for the frame");
    }
}
