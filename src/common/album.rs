//! The photo album on the board's external flash.
//!
//! `cargo proj mbedded photos <folder>` writes it from the PC, starting at
//! the first byte of the chip:
//!
//! | bytes          | what                                         |
//! |----------------|----------------------------------------------|
//! | 0 to 7         | `H7ALBUM1`                                   |
//! | 8 to 11        | number of photos, little-endian              |
//! | 12 to 15       | width and height of each photo (128)         |
//! | 16 to 63       | zero                                         |
//! | 64 onwards     | the photos, one after the other              |
//!
//! Each photo is 128 x 128 RGB565 pixels, low byte first, row by row: the
//! same as the photos `build.rs` bakes into the firmware (`ui::photo`).

/// The first bytes of an album.
pub const MAGIC: [u8; 8] = *b"H7ALBUM1";
/// Bytes before the first photo.
pub const HEADER_BYTES: u32 = 64;
/// Width and height of every photo.
pub const SIDE: u32 = 128;
/// Bytes in one photo.
pub const PHOTO_BYTES: u32 = SIDE * SIDE * 2;
/// The most photos the board loads (2 MB, half a second to load).
pub const MAX_PHOTOS: u32 = 64;

/// An album found on the flash.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Album {
    /// How many photos the board will use (at most [`MAX_PHOTOS`]).
    pub count: u32,
}

impl Album {
    /// Reads an album header. `None` if the bytes are not one (a blank chip,
    /// another format, or no photos).
    pub fn parse(header: &[u8]) -> Option<Self> {
        let word = |at: usize| -> Option<u32> {
            let b = header.get(at..at + 4)?;
            Some(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        };
        if header.get(..8)? != MAGIC || word(12)? != SIDE {
            return None;
        }
        let count = word(8)?.min(MAX_PHOTOS);
        (count > 0).then_some(Self { count })
    }

    /// Where photo `i` starts on the flash.
    pub const fn offset(i: u32) -> u32 {
        HEADER_BYTES + i * PHOTO_BYTES
    }

    /// Bytes of all the photos the board uses.
    pub const fn bytes(&self) -> u32 {
        self.count * PHOTO_BYTES
    }

    /// A header for `count` photos, as the PC writes it.
    pub fn header(count: u32) -> [u8; HEADER_BYTES as usize] {
        let mut h = [0u8; HEADER_BYTES as usize];
        h[..8].copy_from_slice(&MAGIC);
        h[8..12].copy_from_slice(&count.to_le_bytes());
        h[12..16].copy_from_slice(&SIDE.to_le_bytes());
        h
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_header_reads_back() {
        assert_eq!(Album::parse(&Album::header(12)), Some(Album { count: 12 }));
    }

    #[test]
    fn a_blank_chip_or_an_empty_album_is_no_album() {
        assert_eq!(Album::parse(&[0xFF; 64]), None, "erased flash");
        assert_eq!(Album::parse(&[0x00; 64]), None);
        assert_eq!(Album::parse(&Album::header(0)), None, "no photos");
        assert_eq!(Album::parse(&MAGIC), None, "too short");
        let mut other_size = Album::header(3);
        other_size[12] = 64;
        assert_eq!(Album::parse(&other_size), None);
    }

    #[test]
    fn a_huge_album_is_capped() {
        assert_eq!(
            Album::parse(&Album::header(5_000)).unwrap().count,
            MAX_PHOTOS
        );
    }

    #[test]
    fn photos_follow_the_header_one_after_another() {
        assert_eq!(Album::offset(0), 64);
        assert_eq!(Album::offset(1), 64 + 32_768);
        assert_eq!(Album { count: 3 }.bytes(), 3 * 32_768);
    }
}
