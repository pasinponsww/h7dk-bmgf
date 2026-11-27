//! The 16 MB Infineon S70KL1281 HyperRAM on the board's OCTOSPI2.
//!
//! Set up as in the HAL's `octospi_hyperram` example for this board, then
//! switched to memory-mapped mode: the RAM appears at 0x7000_0000 and is read
//! and written like any other memory. Only the CPU uses it (the LCD
//! controller keeps reading the AXI SRAM), and the default memory map caches
//! it, so reading photos from it is quick.

use core::slice;

use stm32h7xx_hal::pac;
use stm32h7xx_hal::prelude::*;
use stm32h7xx_hal::rcc::{rec, CoreClocks};
use stm32h7xx_hal::time::MicroSeconds;
use stm32h7xx_hal::xspi::HyperbusConfig;

/// The RAM's size in bytes.
pub const HYPERRAM_BYTES: usize = 16 * 1024 * 1024;

/// OCTOSPI2 and its clock, waiting to be switched on.
pub struct HyperRamPort {
    pub(crate) ospi: pac::OCTOSPI2,
    pub(crate) rec: rec::Octospi2,
    pub(crate) clocks: CoreClocks,
}

impl HyperRamPort {
    /// Switches the HyperRAM on and checks it: writes two patterns over the
    /// first and last 64 KB and reads them back (more than the 32 KB data
    /// cache holds, so the reads come from the chip). Returns all 16 MB, or
    /// `None` if the check failed. Call it once; it takes the port.
    #[allow(unsafe_code)]
    pub fn start(self) -> Option<&'static mut [u8]> {
        let config = HyperbusConfig::new(80.MHz())
            .device_size_bytes(24) // 2^24 = 16 MB
            .refresh_interval(MicroSeconds::from_ticks(4))
            .read_write_recovery(4) // 50 ns
            .access_initial_latency(6);
        let hyperbus = self
            .ospi
            .octospi_hyperbus_unchecked(config, &self.clocks, self.rec);
        // SAFETY: `init` puts OCTOSPI2 in memory-mapped mode, after which the
        // 16 MB from the returned address are RAM that nothing else uses. The
        // port is consumed, so this slice is the only way to it.
        let ram = unsafe {
            let ptr = hyperbus.init().cast::<u8>();
            slice::from_raw_parts_mut(ptr, HYPERRAM_BYTES)
        };
        works(ram).then_some(ram)
    }
}

/// Bytes checked at each end of the RAM.
const CHECK: usize = 64 * 1024;

/// Writes and reads back two patterns at both ends of `ram`.
fn works(ram: &mut [u8]) -> bool {
    let len = ram.len();
    let ends = [0..CHECK, len - CHECK..len];
    for pattern in [0xA5u8, 0x5A] {
        for r in ends.clone() {
            for (i, b) in ram[r.clone()].iter_mut().enumerate() {
                *b = pattern ^ (i as u8);
            }
        }
        for r in ends.clone() {
            if ram[r]
                .iter()
                .enumerate()
                .any(|(i, &b)| b != pattern ^ (i as u8))
            {
                return false;
            }
        }
    }
    true
}
