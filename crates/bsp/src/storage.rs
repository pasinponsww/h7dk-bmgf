//! The last 128 KB sector of the chip's flash, kept for saved progress.
//!
//! `memory.x` stops the firmware at 896 KB, so nothing else is ever put in
//! this sector. Records are written into empty 32-byte slots (one flash word
//! each) and the sector is erased only when every slot is used (see the
//! app's `save.rs`).
//!
//! The chip has a single flash bank, so the CPU waits while a word is
//! written (microseconds) or the sector is erased (about a second, which
//! happens once every few thousand saves).

use stm32h7xx_hal::flash::{FlashExt as _, LockedFlashBank};
use stm32h7xx_hal::pac;

/// The sector's number.
const SECTOR: u8 = 7;
/// Where it starts in the flash.
const OFFSET: usize = 7 * SECTOR_BYTES;
/// Its size.
pub const SECTOR_BYTES: usize = 0x2_0000;
/// Bytes in one record slot.
pub const SLOT_BYTES: usize = 32;
/// Record slots in the sector.
pub const SLOTS: usize = SECTOR_BYTES / SLOT_BYTES;

/// Writing the flash failed.
#[derive(Copy, Clone, Debug, PartialEq, Eq, defmt::Format)]
pub struct WriteFailed;

/// The save sector.
pub struct SaveSector {
    bank: LockedFlashBank,
}

impl SaveSector {
    pub(crate) fn new(flash: pac::FLASH) -> Self {
        let (bank, _) = flash.split();
        Self { bank }
    }

    /// The sector as it is now, read straight from the flash. Read it at
    /// start-up, before anything is written.
    pub fn bytes(&self) -> &[u8] {
        &self.bank.read_all()[OFFSET..OFFSET + SECTOR_BYTES]
    }

    /// Writes one record into empty slot `slot`.
    pub fn write(&mut self, slot: usize, record: &[u8; SLOT_BYTES]) -> Result<(), WriteFailed> {
        if slot >= SLOTS {
            return Err(WriteFailed);
        }
        self.bank
            .unlocked()
            .program(OFFSET + slot * SLOT_BYTES, record.iter())
            .map_err(|_| WriteFailed)
    }

    /// Erases the whole sector, emptying every slot.
    pub fn erase(&mut self) -> Result<(), WriteFailed> {
        self.bank
            .unlocked()
            .erase_sector(SECTOR)
            .map_err(|_| WriteFailed)
    }
}
