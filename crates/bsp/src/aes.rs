//! The STM32H735's AES engine (the CRYP), in counter mode.
//!
//! Counter mode turns data over the same way to encrypt or decrypt, so no
//! key preparation is needed. Data goes in and out in byte order (the CRYP
//! swaps the bytes of each word for us); the key and the first counter block
//! go in as big-endian words. Every wait is bounded, so a misbehaving engine
//! gives a wrong answer rather than a hang (the app then opens the message
//! in software instead).

use stm32h7xx_hal::pac;
use stm32h7xx_hal::rcc::rec;
use stm32h7xx_hal::rcc::ResetEnable as _;

/// CR: AES in counter mode (ALGOMODE 0b0110), byte data, 128-bit key.
const CR_AES_CTR_BYTES: u32 = 0b110 << 3 | 0b10 << 6;
/// CR: flush the FIFOs.
const CR_FFLUSH: u32 = 1 << 14;
/// CR: switch the engine on.
const CR_CRYPEN: u32 = 1 << 15;
/// How many times to look at the FIFOs before giving up.
const TRIES: u32 = 100_000;

/// The AES engine.
pub struct Aes {
    regs: pac::CRYP,
}

fn be(b: &[u8]) -> u32 {
    u32::from_be_bytes([b[0], b[1], b[2], b[3]])
}

impl Aes {
    pub(crate) fn new(regs: pac::CRYP, rec: rec::Crypt) -> Self {
        rec.enable().reset();
        Self { regs }
    }

    /// Turns `data` over in place with AES-128 in counter mode, counting up
    /// from `iv` (its last 32 bits are the counter).
    #[allow(unsafe_code)]
    pub fn ctr(&mut self, key: &[u8; 16], iv: &[u8; 16], data: &mut [u8]) {
        let r = &self.regs;
        // SAFETY: every value written is a valid setting for these registers,
        // and only this driver uses the CRYP.
        unsafe {
            r.cr.write(|w| w.bits(0));
            r.cr.write(|w| w.bits(CR_AES_CTR_BYTES));
            r.k2lr.write(|w| w.bits(be(&key[0..4])));
            r.k2rr.write(|w| w.bits(be(&key[4..8])));
            r.k3lr.write(|w| w.bits(be(&key[8..12])));
            r.k3rr.write(|w| w.bits(be(&key[12..16])));
            r.iv0lr.write(|w| w.bits(be(&iv[0..4])));
            r.iv0rr.write(|w| w.bits(be(&iv[4..8])));
            r.iv1lr.write(|w| w.bits(be(&iv[8..12])));
            r.iv1rr.write(|w| w.bits(be(&iv[12..16])));
            r.cr.write(|w| w.bits(CR_AES_CTR_BYTES | CR_FFLUSH));
            r.cr.write(|w| w.bits(CR_AES_CTR_BYTES | CR_CRYPEN));
        }
        for block in data.chunks_mut(16) {
            let mut input = [0u8; 16];
            input[..block.len()].copy_from_slice(block);
            for word in input.as_chunks::<4>().0 {
                if !(0..TRIES).any(|_| r.sr.read().ifnf().bit_is_set()) {
                    break;
                }
                let v = u32::from_le_bytes(*word);
                // SAFETY: any word is valid input data.
                r.din.write(|w| unsafe { w.bits(v) });
            }
            let mut output = [0u8; 16];
            for word in output.as_chunks_mut::<4>().0 {
                if !(0..TRIES).any(|_| r.sr.read().ofne().bit_is_set()) {
                    break;
                }
                word.copy_from_slice(&r.dout.read().bits().to_le_bytes());
            }
            block.copy_from_slice(&output[..block.len()]);
        }
        // SAFETY: switching the engine off is always allowed.
        r.cr.write(|w| unsafe { w.bits(0) });
    }
}
