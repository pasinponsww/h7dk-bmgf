//! The 64 MB Macronix MX25LM51245G flash on the board's OCTOSPI1.
//!
//! The chip wakes up in plain 1-bit SPI mode, and this driver leaves it there:
//! it only reads, with the 4-byte-address FAST READ command, 32 bytes (the
//! OCTOSPI FIFO) at a time. At 50 MHz that is about 4 MB a second, plenty to
//! load photos at start-up, and nothing here can change what is stored on
//! the chip. `cargo proj mbedded photos` writes it from the PC.

use stm32h7xx_hal::pac;
use stm32h7xx_hal::xspi::{Octospi, OctospiError, OctospiWord as Word};

/// The chip's size in bytes.
pub const FLASH_BYTES: u32 = 64 * 1024 * 1024;
/// JEDEC id of the MX25LM51245G: Macronix, octal flash, 512 Mbit.
pub const MX25LM51245G_ID: [u8; 3] = [0xC2, 0x85, 0x3A];

/// READ IDENTIFICATION.
const RDID: u8 = 0x9F;
/// FAST READ with a 4-byte address, 8 dummy clocks.
const FAST_READ4B: u8 = 0x0C;
const FAST_READ_DUMMY: u8 = 8;
/// Bytes per OCTOSPI transaction: the size of its FIFO.
const CHUNK: usize = 32;

/// What went wrong reading the flash.
#[derive(Copy, Clone, Debug, PartialEq, Eq, defmt::Format)]
pub enum FlashError {
    /// The read runs past the end of the chip.
    OutOfRange,
    /// The OCTOSPI peripheral reported an error.
    Bus,
}

impl From<OctospiError> for FlashError {
    fn from(_: OctospiError) -> Self {
        Self::Bus
    }
}

/// Reads the external flash.
pub struct ExternalFlash {
    ospi: Octospi<pac::OCTOSPI1>,
}

impl ExternalFlash {
    pub(crate) fn new(ospi: Octospi<pac::OCTOSPI1>) -> Self {
        Self { ospi }
    }

    /// The chip's JEDEC id, or `None` if the read failed. A missing or
    /// unpowered chip reads as all `0xFF` or all `0x00`.
    pub fn id(&mut self) -> Option<[u8; 3]> {
        let mut id = [0u8; 3];
        self.ospi.read(RDID, &mut id).ok()?;
        Some(id)
    }

    /// True if the chip answers with the id this board should have.
    pub fn present(&mut self) -> bool {
        self.id() == Some(MX25LM51245G_ID)
    }

    /// Fills `buf` from the chip, starting at byte `addr`.
    pub fn read(&mut self, addr: u32, buf: &mut [u8]) -> Result<(), FlashError> {
        let len = u32::try_from(buf.len()).map_err(|_| FlashError::OutOfRange)?;
        if addr.checked_add(len).is_none_or(|end| end > FLASH_BYTES) {
            return Err(FlashError::OutOfRange);
        }
        let mut at = addr;
        for chunk in buf.chunks_mut(CHUNK) {
            self.ospi.read_extended(
                Word::U8(FAST_READ4B),
                Word::U32(at),
                Word::None,
                FAST_READ_DUMMY,
                chunk,
            )?;
            at += u32::try_from(chunk.len()).unwrap_or(0);
        }
        Ok(())
    }
}
