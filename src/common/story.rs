//! The story: what the logic needs to know about it, and the story itself.
use geometry::Rect;

/// Read-only view of the story.
pub trait Story {
    /// Number of chapters.
    fn chapter_count(&self) -> usize;
    /// Number of tappable objects in a chapter (at most 8).
    fn hotspot_count(&self, chapter: usize) -> usize;
    /// Where an object sits on the screen.
    fn hotspot_rect(&self, chapter: usize, index: usize) -> Rect;
}

/// Mask with one bit set for each of `n` objects.
pub fn full_mask(n: usize) -> u8 {
    if n >= 8 {
        u8::MAX
    } else {
        (1u8 << n) - 1
    }
}

/// Size of every tappable object, in pixels.
pub const OBJECT_SIZE: i16 = 64;

/// One tappable object on a story page.
#[derive(Copy, Clone, Debug)]
pub struct Hotspot {
    /// Where it sits on the 480 x 272 screen.
    pub rect: Rect,
    /// Name of its icon in `assets/icons/phosphor/duotone`.
    pub icon: &'static str,
    /// The line revealed when it is tapped.
    pub line: &'static str,
}

/// One chapter of the story.
#[derive(Copy, Clone, Debug)]
pub struct Chapter {
    /// Big title at the top of the page.
    pub title: &'static str,
    /// Its tappable objects.
    pub hotspots: &'static [Hotspot],
}

/// Where object `index` goes when the chapter has `count` objects.
/// Positions come from the layout, not from the story file.
const fn slot(count: usize, index: usize) -> (i16, i16) {
    const TWO: [(i16, i16); 2] = [(286, 52), (366, 128)];
    const THREE: [(i16, i16); 3] = [(268, 36), (372, 66), (304, 134)];
    const FOUR: [(i16, i16); 4] = [(262, 30), (372, 52), (290, 112), (384, 138)];
    match count {
        2 => TWO[index],
        3 => THREE[index],
        _ => FOUR[index],
    }
}

const fn hs(count: usize, index: usize, icon: &'static str, line: &'static str) -> Hotspot {
    let (x, y) = slot(count, index);
    Hotspot {
        rect: Rect::new(x, y, OBJECT_SIZE, OBJECT_SIZE),
        icon,
        line,
    }
}

include!(concat!(env!("OUT_DIR"), "/chapters.rs"));

/// The real story, ready to hand to [`crate::update`].
#[derive(Copy, Clone, Debug, Default)]
pub struct Content;

impl Story for Content {
    fn chapter_count(&self) -> usize {
        CHAPTERS.len()
    }
    fn hotspot_count(&self, chapter: usize) -> usize {
        CHAPTERS.get(chapter).map_or(0, |c| c.hotspots.len())
    }
    fn hotspot_rect(&self, chapter: usize, index: usize) -> Rect {
        CHAPTERS
            .get(chapter)
            .and_then(|c| c.hotspots.get(index))
            .map_or(Rect::new(0, 0, 0, 0), |h| h.rect)
    }
}
