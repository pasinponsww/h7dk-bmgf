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

#[cfg(test)]
mod tests {
    use super::{full_mask, Content, Story, CHAPTERS};
    use crate::state::{NEXT_BUTTON, SCREEN_H, SCREEN_W};
    use crate::{update, Event, Model, Point, Scene, TouchEvent, XorShift32};

    #[test]
    fn mask_has_one_bit_per_object() {
        assert_eq!(full_mask(0), 0);
        assert_eq!(full_mask(2), 0b11);
        assert_eq!(full_mask(3), 0b111);
        assert_eq!(full_mask(8), 0xFF);
        assert_eq!(full_mask(20), 0xFF);
    }

    // Checks that hold for any story, so they also guard a personal one:
    // run them with `cargo proj common test --story <name>`.
    #[test]
    fn there_is_at_least_one_chapter() {
        assert!(!CHAPTERS.is_empty());
        assert!(
            CHAPTERS.len() <= 32,
            "the progress trail has room for 32 chapters at most"
        );
    }

    #[test]
    fn every_chapter_has_two_to_four_objects() {
        for c in &CHAPTERS {
            assert!((2..=4).contains(&c.hotspots.len()), "{}", c.title);
        }
    }

    #[test]
    fn objects_fit_on_screen_and_do_not_overlap_each_other_or_next() {
        for c in &CHAPTERS {
            for (i, a) in c.hotspots.iter().enumerate() {
                let r = a.rect;
                assert!(
                    r.x >= 0 && r.y >= 0 && r.x + r.w <= SCREEN_W && r.y + r.h <= SCREEN_H,
                    "{}",
                    c.title
                );
                assert!(
                    !r.overlaps(&NEXT_BUTTON, 0),
                    "{} object {i} covers Next",
                    c.title
                );
                for b in &c.hotspots[i + 1..] {
                    assert!(!r.overlaps(&b.rect, 0), "{}: objects overlap", c.title);
                }
            }
        }
    }

    #[test]
    fn titles_and_lines_are_filled_in_and_not_too_long() {
        for c in &CHAPTERS {
            assert!(!c.title.trim().is_empty());
            for h in c.hotspots {
                assert!(!h.line.trim().is_empty(), "{}: empty line", c.title);
                assert!(h.line.chars().count() <= 160, "{}: line too long", c.title);
                assert!(!h.icon.is_empty());
            }
        }
    }

    #[test]
    fn story_trait_matches_the_data() {
        let s = Content;
        assert_eq!(s.chapter_count(), CHAPTERS.len());
        for (ch, c) in CHAPTERS.iter().enumerate() {
            assert_eq!(s.hotspot_count(ch), c.hotspots.len());
            for (i, h) in c.hotspots.iter().enumerate() {
                assert_eq!(s.hotspot_rect(ch, i), h.rect);
            }
        }
        assert_eq!(s.hotspot_count(usize::MAX), 0, "out of range is safe");
    }

    #[test]
    fn full_playthrough_reaches_the_celebration() {
        let story = Content;
        let mut m = Model::new(0);
        let mut rng = XorShift32::new(99);
        let mut tap = |m: &mut Model, p: Point| {
            update(m, &story, &mut rng, Event::Touch(TouchEvent::Down(p)));
        };
        tap(&mut m, Point::new(10, 10));
        for ch in 0..story.chapter_count() {
            for i in 0..story.hotspot_count(ch) {
                let r = story.hotspot_rect(ch, i);
                tap(&mut m, Point::new(r.x + 5, r.y + 5));
            }
            tap(&mut m, Point::new(NEXT_BUTTON.x + 5, NEXT_BUTTON.y + 5));
        }
        assert_eq!(m.scene, Scene::Question);
        let yes = m.yes_rect();
        tap(&mut m, Point::new(yes.x + 5, yes.y + 5));
        assert_eq!(m.scene, Scene::Celebration);
    }
}
