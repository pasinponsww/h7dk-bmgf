//! How long frames take, summed up once a second for the log.
//!
//! The render task measures each frame in CPU cycles: the time spent drawing
//! into the band buffer and the time spent copying bands into the
//! framebuffer. This turns those into one log line a second, so every speed-up
//! can be checked on the board.

/// How often a report is made.
pub const REPORT_EVERY_MS: u32 = 1_000;

/// One second of frames.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Report {
    /// Frames finished in the window.
    pub frames: u32,
    /// Average time spent drawing one frame, in microseconds.
    pub draw_avg_us: u32,
    /// Longest time spent drawing one frame, in microseconds.
    pub draw_max_us: u32,
    /// Average time spent copying one frame to the framebuffer, in microseconds.
    pub copy_avg_us: u32,
    /// Share of the window the CPU spent drawing and copying, in percent.
    pub busy_pct: u32,
}

/// Collects frame times until a report is due.
#[derive(Copy, Clone, Debug)]
pub struct FrameStats {
    cycles_per_us: u32,
    window_start_ms: u32,
    frames: u32,
    draw_sum: u64,
    draw_max: u32,
    copy_sum: u64,
}

impl FrameStats {
    /// Starts a window at `now_ms`, for a CPU running at `core_hz`.
    pub const fn new(core_hz: u32, now_ms: u32) -> Self {
        let per_us = core_hz / 1_000_000;
        Self::window(if per_us == 0 { 1 } else { per_us }, now_ms)
    }

    const fn window(cycles_per_us: u32, now_ms: u32) -> Self {
        Self {
            cycles_per_us,
            window_start_ms: now_ms,
            frames: 0,
            draw_sum: 0,
            draw_max: 0,
            copy_sum: 0,
        }
    }

    /// Adds one frame that took `draw` cycles to draw and `copy` cycles to
    /// copy, finished at `now_ms`. Returns a report once a window is full,
    /// and starts the next window.
    pub fn record(&mut self, draw: u32, copy: u32, now_ms: u32) -> Option<Report> {
        self.frames += 1;
        self.draw_sum += u64::from(draw);
        self.draw_max = self.draw_max.max(draw);
        self.copy_sum += u64::from(copy);

        let elapsed = now_ms.wrapping_sub(self.window_start_ms);
        if elapsed < REPORT_EVERY_MS {
            return None;
        }
        let us =
            |cycles: u64| u32::try_from(cycles / u64::from(self.cycles_per_us)).unwrap_or(u32::MAX);
        let frames = u64::from(self.frames);
        let busy_us = us(self.draw_sum + self.copy_sum);
        let report = Report {
            frames: self.frames,
            draw_avg_us: us(self.draw_sum / frames),
            draw_max_us: us(u64::from(self.draw_max)),
            copy_avg_us: us(self.copy_sum / frames),
            busy_pct: busy_us / (elapsed.max(1) * 10),
        };
        *self = Self::window(self.cycles_per_us, now_ms);
        Some(report)
    }
}

#[cfg(test)]
mod tests {
    use super::{FrameStats, Report, REPORT_EVERY_MS};

    const HZ: u32 = 200_000_000;

    #[test]
    fn nothing_is_reported_before_a_second_has_passed() {
        let mut s = FrameStats::new(HZ, 0);
        for t in (33..REPORT_EVERY_MS).step_by(33) {
            assert_eq!(s.record(1_000, 100, t), None);
        }
    }

    #[test]
    fn a_report_sums_up_the_window_then_starts_over() {
        let mut s = FrameStats::new(HZ, 0);
        // 200 cycles is 1 us at 200 MHz.
        assert_eq!(s.record(2_000_000, 200_000, 400), None);
        assert_eq!(s.record(4_000_000, 200_000, 700), None);
        let r = s.record(3_000_000, 200_000, 1_000).unwrap();
        assert_eq!(
            r,
            Report {
                frames: 3,
                draw_avg_us: 15_000,
                draw_max_us: 20_000,
                copy_avg_us: 1_000,
                busy_pct: 4,
            }
        );
        assert_eq!(s.record(1, 1, 1_500), None, "a new window has started");
        let r = s.record(200, 0, 2_000).unwrap();
        assert_eq!(r.frames, 2);
        assert_eq!(r.draw_max_us, 1);
    }

    #[test]
    fn busy_is_the_share_of_the_window_spent_working() {
        let mut s = FrameStats::new(HZ, 0);
        // Half a second of work in a one second window.
        let r = s.record(100_000_000, 0, 1_000).unwrap();
        assert_eq!(r.busy_pct, 50);
    }

    #[test]
    fn a_zero_clock_does_not_divide_by_zero() {
        let mut s = FrameStats::new(0, 0);
        assert!(s.record(10, 10, REPORT_EVERY_MS).is_some());
    }
}
