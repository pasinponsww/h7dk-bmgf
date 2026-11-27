//! How warm the board is, and how the dog feels about it.
//!
//! The chip's temperature sensor gives a raw reading; the chip also keeps
//! two factory readings, taken at 30 °C and 110 °C. A straight line through
//! those two turns any reading into degrees. The silicon runs a little above
//! the room, so the dog only reacts at the ends: shivering in the cold,
//! panting in the heat.

/// Below this (silicon, °C) the dog shivers.
pub const COLD_BELOW: i16 = 25;
/// Above this the dog pants and sweats.
pub const HOT_ABOVE: i16 = 62;

/// How the dog feels about the temperature.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Feel {
    /// Shivering.
    Cold,
    /// Just right.
    Comfy,
    /// Panting.
    Hot,
}

/// Degrees Celsius from a raw reading and the factory readings at 30 and
/// 110 °C. The ADC may read at 12 bits while the factory readings are 16-bit
/// (or the other way round), so the reading is first put on the same scale.
/// `None` if the calibration is missing.
pub fn celsius(raw: u32, cal30: u16, cal110: u16) -> Option<i16> {
    let (c30, c110) = (i64::from(cal30), i64::from(cal110));
    if c110 <= c30 {
        return None;
    }
    let raw = i64::from(raw);
    let raw = match (c30 > 4_095, raw > 4_095) {
        (true, false) => raw * 16,
        (false, true) => raw / 16,
        _ => raw,
    };
    let t = 30 + (raw - c30) * 80 / (c110 - c30);
    i16::try_from(t.clamp(-100, 200)).ok()
}

/// How the dog feels at `celsius`, or comfy when the temperature is unknown.
pub fn feel(celsius: Option<i16>) -> Feel {
    match celsius {
        Some(t) if t < COLD_BELOW => Feel::Cold,
        Some(t) if t > HOT_ABOVE => Feel::Hot,
        _ => Feel::Comfy,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Typical 16-bit factory readings.
    const CAL30: u16 = 12_400;
    const CAL110: u16 = 16_400;

    #[test]
    fn the_calibration_points_read_as_their_temperatures() {
        assert_eq!(celsius(u32::from(CAL30), CAL30, CAL110), Some(30));
        assert_eq!(celsius(u32::from(CAL110), CAL30, CAL110), Some(110));
        assert_eq!(celsius(14_400, CAL30, CAL110), Some(70), "half way");
        assert_eq!(celsius(12_000, CAL30, CAL110), Some(22), "below 30");
    }

    #[test]
    fn a_12_bit_reading_is_scaled_to_16_bit_calibration() {
        assert_eq!(celsius(14_400 / 16, CAL30, CAL110), Some(70));
        assert_eq!(celsius(900, 775, 1_025), Some(70), "both 12-bit");
        assert_eq!(
            celsius(14_400, 775, 1_025),
            Some(70),
            "16-bit reading, 12-bit calibration"
        );
    }

    #[test]
    fn missing_calibration_gives_no_temperature() {
        assert_eq!(celsius(1_000, 0, 0), None);
        assert_eq!(celsius(1_000, 0xFFFF, 0xFFFF), None);
    }

    #[test]
    fn the_dog_reacts_only_at_the_ends() {
        assert_eq!(feel(Some(10)), Feel::Cold);
        assert_eq!(feel(Some(40)), Feel::Comfy);
        assert_eq!(feel(Some(70)), Feel::Hot);
        assert_eq!(feel(None), Feel::Comfy);
    }
}
