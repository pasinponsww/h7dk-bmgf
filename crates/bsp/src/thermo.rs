//! The chip's own temperature sensor, read with ADC3.
//!
//! The reading is turned into degrees by the app (`app::warmth`), from the
//! two factory calibration readings the chip keeps (taken at 30 and 110 °C
//! with a 3.3 V supply, as the STM32H735G-DK has). It is the silicon's
//! temperature, a little above the room's.

use stm32h7xx_hal::adc::{self, AdcSampleTime, Enabled, Temperature};
use stm32h7xx_hal::hal::adc::OneShot as _;
use stm32h7xx_hal::hal::blocking::delay::DelayUs;
use stm32h7xx_hal::pac;
use stm32h7xx_hal::prelude::*;
use stm32h7xx_hal::rcc::rec;
use stm32h7xx_hal::rcc::CoreClocks;
use stm32h7xx_hal::signature::{TS_CAL_110, TS_CAL_30};

/// A busy-wait delay for setting the ADC up: SysTick belongs to RTIC.
struct Spin {
    cycles_per_us: u32,
}

impl DelayUs<u8> for Spin {
    fn delay_us(&mut self, us: u8) {
        cortex_m::asm::delay(u32::from(us) * self.cycles_per_us);
    }
}

/// The temperature sensor.
pub struct Thermometer {
    adc: adc::Adc<pac::ADC3, Enabled>,
    channel: Temperature,
}

impl Thermometer {
    /// Sets the ADC up. The ADC kernel clock must already be per_ck (see
    /// `Board::init`).
    pub(crate) fn new(adc3: pac::ADC3, rec: rec::Adc3, clocks: &CoreClocks) -> Self {
        let mut delay = Spin {
            cycles_per_us: clocks.c_ck().raw() / 1_000_000,
        };
        let mut adc = adc::Adc::adc3(adc3, 4.MHz(), &mut delay, rec, clocks);
        // The sensor needs a long sample time to settle.
        adc.set_sample_time(AdcSampleTime::T_387);
        let mut channel = Temperature::new();
        channel.enable(&adc);
        delay.delay_us(25);
        Self {
            adc: adc.enable(),
            channel,
        }
    }

    /// One raw reading of the sensor, or `None` if the ADC failed.
    pub fn read(&mut self) -> Option<u32> {
        self.adc.read(&mut self.channel).ok()
    }

    /// The factory readings at 30 °C and at 110 °C.
    pub fn calibration() -> (u16, u16) {
        (TS_CAL_30::read(), TS_CAL_110::read())
    }
}
