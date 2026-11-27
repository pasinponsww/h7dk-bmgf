//! What survives a power-off: where the story was, and how many times she
//! has said yes.
//!
//! The firmware keeps it in the last 128 KB sector of the chip's flash, as
//! 32-byte records (one flash word each) written one after another. Flash
//! can only be erased a whole sector at a time, so a new record never
//! rewrites an old one: it goes in the next empty slot, and the newest valid
//! record wins. Only when all 4096 slots are used is the sector erased.

/// Bytes in one record: one flash word.
pub const RECORD_BYTES: usize = 32;
/// The first bytes of a record.
const MAGIC: [u8; 4] = *b"SAV1";

/// Where to pick the story up again.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub enum Resume {
    /// From the beginning.
    #[default]
    Start,
    /// At this chapter (zero-based).
    Chapter(u8),
    /// At the question (after the last chapter).
    Question,
}

/// What is saved.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Progress {
    /// Where the story was.
    pub resume: Resume,
    /// How many times she has said yes.
    pub yes_count: u32,
}

fn checksum(bytes: &[u8]) -> u32 {
    // FNV-1a.
    bytes.iter().fold(0x811c_9dc5u32, |h, &b| {
        (h ^ u32::from(b)).wrapping_mul(0x0100_0193)
    })
}

impl Progress {
    /// The record for this progress, numbered `seq`.
    pub fn encode(&self, seq: u32) -> [u8; RECORD_BYTES] {
        let mut r = [0u8; RECORD_BYTES];
        r[..4].copy_from_slice(&MAGIC);
        r[4..8].copy_from_slice(&seq.to_le_bytes());
        r[8..12].copy_from_slice(&self.yes_count.to_le_bytes());
        let (kind, chapter) = match self.resume {
            Resume::Start => (0, 0),
            Resume::Chapter(c) => (1, c),
            Resume::Question => (2, 0),
        };
        r[12] = kind;
        r[13] = chapter;
        let sum = checksum(&r[..28]);
        r[28..].copy_from_slice(&sum.to_le_bytes());
        r
    }

    /// The progress and number in a record, if it is a whole, valid one.
    pub fn decode(r: &[u8]) -> Option<(u32, Self)> {
        let word = |at: usize| u32::from_le_bytes([r[at], r[at + 1], r[at + 2], r[at + 3]]);
        if r.len() != RECORD_BYTES || r[..4] != MAGIC || word(28) != checksum(&r[..28]) {
            return None;
        }
        let resume = match (r[12], r[13]) {
            (0, _) => Resume::Start,
            (1, c) => Resume::Chapter(c),
            (2, _) => Resume::Question,
            _ => return None,
        };
        Some((
            word(4),
            Self {
                resume,
                yes_count: word(8),
            },
        ))
    }
}

/// What a scan of the save sector found.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Found {
    /// The newest progress, or the default for an empty sector.
    pub progress: Progress,
    /// Its number; the next record gets the one after.
    pub seq: u32,
    /// The first empty slot, or `None` if the sector is full.
    pub free: Option<usize>,
}

/// Reads the save sector `sector` (erased flash reads as all `0xFF`).
pub fn scan(sector: &[u8]) -> Found {
    let mut best: Option<(u32, Progress)> = None;
    let mut free = None;
    let (slots, _) = sector.as_chunks::<RECORD_BYTES>();
    for (i, slot) in slots.iter().enumerate() {
        if slot.iter().all(|&b| b == 0xFF) {
            free.get_or_insert(i);
            continue;
        }
        if let Some((seq, p)) = Progress::decode(slot) {
            if best.is_none_or(|(s, _)| seq > s) {
                best = Some((seq, p));
            }
        }
    }
    let (seq, progress) = best.unwrap_or_default();
    Found {
        progress,
        seq,
        free,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SLOTS: usize = 8;

    fn blank() -> [u8; SLOTS * RECORD_BYTES] {
        [0xFF; SLOTS * RECORD_BYTES]
    }

    fn put(sector: &mut [u8], slot: usize, seq: u32, p: Progress) {
        sector[slot * RECORD_BYTES..(slot + 1) * RECORD_BYTES].copy_from_slice(&p.encode(seq));
    }

    #[test]
    fn a_record_reads_back() {
        for resume in [Resume::Start, Resume::Chapter(3), Resume::Question] {
            let p = Progress {
                resume,
                yes_count: 7,
            };
            assert_eq!(Progress::decode(&p.encode(42)), Some((42, p)));
        }
    }

    #[test]
    fn a_damaged_or_half_written_record_is_ignored() {
        let mut r = Progress::default().encode(1);
        r[9] ^= 1;
        assert_eq!(Progress::decode(&r), None, "checksum");
        assert_eq!(Progress::decode(&[0xFF; RECORD_BYTES]), None, "erased");
        assert_eq!(Progress::decode(&r[..16]), None, "too short");
    }

    #[test]
    fn an_empty_sector_starts_from_nothing() {
        let found = scan(&blank());
        assert_eq!(found.progress, Progress::default());
        assert_eq!(found.free, Some(0));
    }

    #[test]
    fn the_newest_record_wins_and_the_next_slot_is_free() {
        let mut s = blank();
        let old = Progress {
            resume: Resume::Chapter(1),
            yes_count: 0,
        };
        let new = Progress {
            resume: Resume::Start,
            yes_count: 1,
        };
        put(&mut s, 0, 5, old);
        put(&mut s, 1, 6, new);
        let found = scan(&s);
        assert_eq!((found.progress, found.seq, found.free), (new, 6, Some(2)));
    }

    #[test]
    fn a_full_sector_has_no_free_slot() {
        let mut s = blank();
        for i in 0..SLOTS {
            put(&mut s, i, u32::try_from(i).unwrap(), Progress::default());
        }
        assert_eq!(scan(&s).free, None);
        assert_eq!(scan(&s).seq, u32::try_from(SLOTS - 1).unwrap());
    }
}
