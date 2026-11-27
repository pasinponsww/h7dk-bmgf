//! The secret message: sealed with AES when the firmware is built, and only
//! opened once she has said yes.
//!
//! `build.rs` encrypts "LOVE" followed by the story's `[celebration]
//! secret` with AES-128 in counter mode, so the text is not in the firmware
//! as plain text. When the celebration starts, the board opens it with the
//! chip's own AES engine (the CRYP); the "LOVE" in front proves it came out
//! right. Should the engine ever give a wrong answer, [`SoftAes`] opens it
//! in software instead.
//!
//! This hides the message from anyone reading the firmware's text, nothing
//! more: the key is in the firmware too.

use aes::cipher::{KeyIvInit, StreamCipher};

/// The most bytes a secret may have.
pub const MAX_SECRET: usize = 90;
/// What every sealed message starts with once opened.
pub const MARK: &[u8; 4] = b"LOVE";

/// A message as it is stored: encrypted, with its key and starting counter.
#[derive(Copy, Clone, Debug)]
pub struct Sealed {
    /// AES-128 key.
    pub key: [u8; 16],
    /// The first counter block.
    pub iv: [u8; 16],
    /// "LOVE" and the message, encrypted.
    pub bytes: &'static [u8],
}

/// Something that runs AES-128 in counter mode (encrypting and decrypting
/// are the same thing in this mode).
pub trait Cipher {
    /// Turns `data` over in place with `key`, counting up from `iv` (the
    /// last 32 bits are the counter).
    fn ctr(&mut self, key: &[u8; 16], iv: &[u8; 16], data: &mut [u8]);
}

/// AES in software (RustCrypto's `aes` and `ctr`).
#[derive(Copy, Clone, Debug, Default)]
pub struct SoftAes;

impl Cipher for SoftAes {
    fn ctr(&mut self, key: &[u8; 16], iv: &[u8; 16], data: &mut [u8]) {
        let mut c = ctr::Ctr32BE::<aes::Aes128>::new(key.into(), iv.into());
        c.apply_keystream(data);
    }
}

/// An opened message.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Opened {
    buf: [u8; MAX_SECRET],
    len: usize,
}

impl Opened {
    /// The message.
    pub fn as_str(&self) -> &str {
        core::str::from_utf8(&self.buf[..self.len]).unwrap_or("")
    }
}

/// Opens `sealed` with `cipher`. `None` if what comes out does not start
/// with "LOVE" or is not text (a wrong key or a misbehaving engine).
pub fn open(sealed: &Sealed, cipher: &mut dyn Cipher) -> Option<Opened> {
    let n = sealed.bytes.len();
    if n < MARK.len() || n > MARK.len() + MAX_SECRET {
        return None;
    }
    let mut work = [0u8; MAX_SECRET + 4];
    work[..n].copy_from_slice(sealed.bytes);
    cipher.ctr(&sealed.key, &sealed.iv, &mut work[..n]);
    if &work[..4] != MARK {
        return None;
    }
    let text = core::str::from_utf8(&work[4..n]).ok()?;
    let mut buf = [0u8; MAX_SECRET];
    buf[..text.len()].copy_from_slice(text.as_bytes());
    Some(Opened {
        buf,
        len: text.len(),
    })
}

#[cfg(test)]
mod tests {
    extern crate std;

    use std::boxed::Box;

    use super::*;

    fn seal(text: &str, key: [u8; 16], iv: [u8; 16]) -> ([u8; MAX_SECRET + 4], usize) {
        let mut buf = [0u8; MAX_SECRET + 4];
        buf[..4].copy_from_slice(MARK);
        buf[4..4 + text.len()].copy_from_slice(text.as_bytes());
        let n = 4 + text.len();
        SoftAes.ctr(&key, &iv, &mut buf[..n]);
        (buf, n)
    }

    #[test]
    fn a_sealed_message_opens() {
        let (key, iv) = ([7u8; 16], [1u8; 16]);
        let (buf, n) = seal("I love you more than pixels", key, iv);
        let bytes: &'static [u8] = Box::leak(buf[..n].to_vec().into_boxed_slice());
        assert!(
            !bytes.windows(4).any(|w| w == b"love"),
            "not readable sealed"
        );
        let sealed = Sealed { key, iv, bytes };
        let opened = open(&sealed, &mut SoftAes).unwrap();
        assert_eq!(opened.as_str(), "I love you more than pixels");
    }

    #[test]
    fn the_wrong_key_opens_nothing() {
        let (buf, n) = seal("hello", [7; 16], [1; 16]);
        let bytes: &'static [u8] = Box::leak(buf[..n].to_vec().into_boxed_slice());
        let wrong = Sealed {
            key: [8; 16],
            iv: [1; 16],
            bytes,
        };
        assert_eq!(open(&wrong, &mut SoftAes), None);
    }

    #[test]
    fn counter_mode_matches_the_standard() {
        // NIST SP 800-38A, F.5.1 (CTR-AES128), first block.
        let key = [
            0x2b, 0x7e, 0x15, 0x16, 0x28, 0xae, 0xd2, 0xa6, 0xab, 0xf7, 0x15, 0x88, 0x09, 0xcf,
            0x4f, 0x3c,
        ];
        let iv = [
            0xf0, 0xf1, 0xf2, 0xf3, 0xf4, 0xf5, 0xf6, 0xf7, 0xf8, 0xf9, 0xfa, 0xfb, 0xfc, 0xfd,
            0xfe, 0xff,
        ];
        let mut block = [
            0x6b, 0xc1, 0xbe, 0xe2, 0x2e, 0x40, 0x9f, 0x96, 0xe9, 0x3d, 0x7e, 0x11, 0x73, 0x93,
            0x17, 0x2a,
        ];
        SoftAes.ctr(&key, &iv, &mut block);
        assert_eq!(
            block,
            [
                0x87, 0x4d, 0x61, 0x91, 0xb6, 0x20, 0xe3, 0x26, 0x1b, 0xef, 0x68, 0x64, 0x99, 0x0d,
                0xb6, 0xce
            ]
        );
    }
}
