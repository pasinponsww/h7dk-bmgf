//! The real-time clock, for the date.
//!
//! It runs from the board's 32.768 kHz crystal (LSE) when that starts, else
//! from the chip's own, rougher, low-speed oscillator (LSI). It keeps the
//! time across resets. Across a power-off it keeps it only if the chip's
//! VBAT pin stays powered; otherwise the firmware sets it again from the
//! time it was built.

use chrono::{Datelike, NaiveDate, Timelike};
use stm32h7xx_hal::pac;
use stm32h7xx_hal::prelude::*;
use stm32h7xx_hal::rcc::backup::BackupREC;
use stm32h7xx_hal::rcc::CoreClocks;
use stm32h7xx_hal::rtc::{Rtc, RtcClock};

/// Backup register 0 holds this once the clock has been set.
const SET_MAGIC: u32 = 0x4C4F_5645;
/// How many times to look at the crystal before giving up on it (about a
/// second at 400 MHz; a crystal can take that long to start).
const LSE_TRIES: u32 = 40_000_000;

/// A moment, as the clock keeps it.
#[derive(Copy, Clone, Debug, PartialEq, Eq, defmt::Format)]
pub struct Moment {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

/// The real-time clock.
pub struct Clock {
    rtc: Rtc,
}

/// Starts the crystal and waits for it, but not forever: some boards have
/// no crystal fitted. True once it runs.
#[allow(unsafe_code)]
fn start_crystal() -> bool {
    // SAFETY: the backup domain is open for writing (the HAL did that when
    // it made the `BackupREC`), and only the clock uses the LSE.
    let rcc = unsafe { &*pac::RCC::ptr() };
    if rcc.bdcr.read().lserdy().is_ready() {
        return true;
    }
    rcc.bdcr.modify(|_, w| w.lseon().on());
    for _ in 0..LSE_TRIES {
        if rcc.bdcr.read().lserdy().is_ready() {
            return true;
        }
    }
    rcc.bdcr.modify(|_, w| w.lseon().off());
    false
}

impl Clock {
    /// Opens the clock if it is running, else starts it (from the crystal
    /// if it starts, else from the LSI).
    pub(crate) fn start(rtc: pac::RTC, backup: BackupREC, clocks: &CoreClocks) -> Self {
        let source = if start_crystal() {
            RtcClock::Lse {
                freq: 32_768.Hz(),
                bypass: false,
                css: false,
            }
        } else {
            defmt::warn!("clock: no 32.768 kHz crystal, using the LSI");
            RtcClock::Lsi
        };
        Self {
            rtc: Rtc::open_or_init(rtc, backup.RTC, source, clocks),
        }
    }

    /// The time now, once the clock has been set.
    pub fn now(&self) -> Option<Moment> {
        if self.rtc.read_backup_reg(0) != SET_MAGIC {
            return None;
        }
        let t = self.rtc.date_time()?;
        Some(Moment {
            year: u16::try_from(t.year()).ok()?,
            month: u8::try_from(t.month()).ok()?,
            day: u8::try_from(t.day()).ok()?,
            hour: u8::try_from(t.hour()).ok()?,
            minute: u8::try_from(t.minute()).ok()?,
            second: u8::try_from(t.second()).ok()?,
        })
    }

    /// Sets the clock. Does nothing for a moment that is not a real one.
    pub fn set(&mut self, m: Moment) {
        let Some(t) =
            NaiveDate::from_ymd_opt(i32::from(m.year), u32::from(m.month), u32::from(m.day))
                .and_then(|d| {
                    d.and_hms_opt(u32::from(m.hour), u32::from(m.minute), u32::from(m.second))
                })
        else {
            return;
        };
        self.rtc.set_date_time(t);
        self.rtc.write_backup_reg(0, SET_MAGIC);
    }
}
