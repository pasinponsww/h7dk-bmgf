//! The application state and its tuning constants. In the firmware the
//! `Model` is an RTIC shared resource: every task reads and writes it, and RTIC
//! makes sure two tasks never do so at the same time.

use geometry::{Point, Rect};

use crate::story::{full_mask, Story};

/// Logical screen width in pixels (the 4.3" LCD on the STM32H735G-DK).
pub const SCREEN_W: i16 = 480;
/// Logical screen height in pixels.
pub const SCREEN_H: i16 = 272;

/// Idle time before an unfound object pulses as a hint.
pub const NUDGE_AFTER_MS: u32 = 3_000;
/// Minimum time between two hints.
pub const NUDGE_EVERY_MS: u32 = 3_000;
/// Idle time after which "Next" unlocks by itself.
pub const UNLOCK_AFTER_MS: u32 = 9_000;
/// How long both LEDs stay on after a tap on an object.
pub const FLASH_MS: u32 = 350;
/// A touch this close to the NO button makes it jump (pixels).
pub const DODGE_RADIUS: i16 = 40;
/// After this many jumps the NO button gives up.
pub const MAX_DODGES: u8 = 5;
/// Width of the NO button once it has given up.
pub const GIVE_UP_W: i16 = 120;
/// The celebration screen returns to the intro after this much idle time.
pub const IDLE_RESET_MS: u32 = 60_000;
/// Where the "Next" button is on story pages.
pub const NEXT_BUTTON: Rect = Rect::new(356, 214, 100, 40);
/// The "Start again" button on the celebration screen.
pub const RESTART_BUTTON: Rect = Rect::new(260, 206, 160, 48);
/// Where the NO button starts on the question screen.
pub const INITIAL_NO: Rect = Rect::new(300, 110, 96, 44);
/// The part of the screen the NO button may jump around in. It stays below
/// the question text and away from the edges.
pub const NO_AREA: Rect = Rect::new(10, 104, 460, 158);
/// Gap kept between the NO and YES buttons when NO jumps.
pub const YES_PAD: i16 = 24;
/// How many random spots NO tries on each jump.
pub const JUMP_TRIES: usize = 64;
/// How long the dog stays in love after being petted.
pub const PET_MS: u32 = 1_800;
/// After this long untouched on the intro, the dog starts playing to get
/// attention.
pub const ATTRACT_AFTER_MS: u32 = 30_000;
/// A finished heart stays on screen this long before the question opens.
pub const TRACE_ADMIRE_MS: u32 = 1_400;
/// Where a chapter's polaroid sits once every object is found: over the
/// objects, a white frame round a 128 x 128 photo with room for a caption.
pub const PHOTO_CARD: Rect = Rect::new(286, 24, 144, 172);
/// The dog sprite's size in sprite pixels (see `ui::mascot`).
const DOG_W: i16 = 34;
const DOG_H: i16 = 40;

/// Where the dog sits in a scene: its top-left corner and how many screen
/// pixels wide each sprite pixel is.
pub const fn dog_spot(scene: Scene) -> (i16, i16, u8) {
    match scene {
        Scene::Intro => (40, 56, 4),
        Scene::Story { .. } => (74, 72, 3),
        Scene::Trace { .. } => (28, 96, 3),
        Scene::Question => (396, 14, 2),
        Scene::Celebration => (36, 56, 4),
    }
}

/// The part of the screen the dog covers in a scene: tapping it pets the dog.
pub const fn dog_rect(scene: Scene) -> Rect {
    spot_rect(dog_spot(scene))
}

const fn spot_rect((x, y, scale): (i16, i16, u8)) -> Rect {
    let s = scale as i16;
    Rect::new(x, y, DOG_W * s, DOG_H * s)
}

/// Where the album slideshow's polaroid sits on the celebration, in the
/// dog's usual place; the dog sits smaller at its corner.
pub const SLIDESHOW_CARD: Rect = Rect::new(24, 26, 144, 172);

/// The screen the user is on.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Scene {
    /// "For you" with an Open button.
    Intro,
    /// A story chapter with tappable objects.
    Story {
        /// Zero-based chapter number.
        chapter: u8,
        /// Bitmask of objects already tapped.
        found: u8,
        /// The object whose line is currently shown.
        shown: Option<u8>,
    },
    /// Draw a heart over a dotted outline, before the question.
    Trace {
        /// Bitmask of the dots traced so far (see `crate::trace`).
        hit: u32,
    },
    /// "Will you be my girlfriend?"
    Question,
    /// The happy ending.
    Celebration,
}

/// Everything the logic remembers.
#[derive(Copy, Clone, Debug)]
pub struct Model {
    /// Current screen.
    pub scene: Scene,
    /// Time of the latest tick, milliseconds since boot.
    pub now_ms: u32,
    /// Where the NO button is.
    pub no_rect: Rect,
    /// How many times NO has jumped.
    pub dodges: u8,
    /// "Next" is unlocked even though not every object was found.
    pub skip_ok: bool,
    pub(crate) scene_start_ms: u32,
    pub(crate) last_touch_ms: u32,
    pub(crate) last_nudge_ms: u32,
    pub(crate) flash_until_ms: u32,
    /// When the dog was last petted, while it is still happy about it.
    pub(crate) petted_ms: Option<u32>,
    /// Where the finger was at the last reading while tracing, if it is down.
    pub(crate) pen: Option<Point>,
    /// When the heart was finished.
    pub(crate) traced_ms: Option<u32>,
    /// When the chapter's polaroid came out, while it is showing.
    pub(crate) photo_ms: Option<u32>,
    /// Photos in the album loaded from the external flash (0 for none). The
    /// celebration shows them as a slideshow.
    pub album: u16,
}

impl Model {
    /// Starts on the intro screen at the given time.
    pub const fn new(now_ms: u32) -> Self {
        Self {
            scene: Scene::Intro,
            now_ms,
            no_rect: INITIAL_NO,
            dodges: 0,
            skip_ok: false,
            scene_start_ms: now_ms,
            last_touch_ms: now_ms,
            last_nudge_ms: now_ms,
            flash_until_ms: 0,
            petted_ms: None,
            pen: None,
            traced_ms: None,
            photo_ms: None,
            album: 0,
        }
    }

    /// Switches scene and resets the per-scene timers.
    pub(crate) fn go(&mut self, scene: Scene) {
        self.scene = scene;
        self.scene_start_ms = self.now_ms;
        self.last_touch_ms = self.now_ms;
        self.last_nudge_ms = self.now_ms;
        self.skip_ok = false;
        self.flash_until_ms = 0;
        self.petted_ms = None;
        self.pen = None;
        self.traced_ms = None;
        self.photo_ms = None;
        if scene == Scene::Question {
            self.dodges = 0;
            self.no_rect = INITIAL_NO;
        }
    }

    /// Where the YES button is. It grows 6% per dodge, up to 150%.
    pub fn yes_rect(&self) -> Rect {
        yes_rect(self.dodges)
    }

    /// Milliseconds since the dog was petted, while it is still in love
    /// (less than [`PET_MS`] ago).
    pub fn petted_for(&self, now_ms: u32) -> Option<u32> {
        let since = now_ms.wrapping_sub(self.petted_ms?);
        (since < PET_MS).then_some(since)
    }

    /// Where the dog sits right now: as [`dog_spot`] says, except on a
    /// celebration with a slideshow, where it sits smaller at the corner of
    /// the polaroid.
    pub const fn dog_spot(&self) -> (i16, i16, u8) {
        match self.scene {
            Scene::Celebration if self.album > 0 => (150, 160, 2),
            scene => dog_spot(scene),
        }
    }

    /// The part of the screen the dog covers right now.
    pub const fn dog_rect(&self) -> Rect {
        spot_rect(self.dog_spot())
    }

    /// Milliseconds since the chapter's polaroid came out, while it shows.
    pub fn photo_for(&self, now_ms: u32) -> Option<u32> {
        Some(now_ms.wrapping_sub(self.photo_ms?))
    }

    /// Milliseconds since the heart was finished, if it has been.
    pub fn traced_for(&self, now_ms: u32) -> Option<u32> {
        Some(now_ms.wrapping_sub(self.traced_ms?))
    }

    /// Milliseconds the dog has been playing for attention at `now_ms`: the
    /// intro has gone untouched for [`ATTRACT_AFTER_MS`] or more.
    pub fn attracting_for(&self, now_ms: u32) -> Option<u32> {
        if self.scene != Scene::Intro {
            return None;
        }
        now_ms
            .wrapping_sub(self.last_touch_ms)
            .checked_sub(ATTRACT_AFTER_MS)
    }

    /// True once the NO button has stopped running away.
    pub fn no_gave_up(&self) -> bool {
        self.dodges >= MAX_DODGES
    }

    /// True if the "Next" button works on the current story page: every
    /// object is found, or the page has been idle long enough.
    pub fn next_unlocked<S: Story>(&self, story: &S) -> bool {
        match self.scene {
            Scene::Story { chapter, found, .. } => {
                self.skip_ok || found == full_mask(story.hotspot_count(usize::from(chapter)))
            }
            Scene::Trace { .. } => self.skip_ok,
            _ => false,
        }
    }
}

/// Position and size of the YES button after `dodges` jumps by NO.
pub fn yes_rect(dodges: u8) -> Rect {
    let pct = (100 + 6 * i32::from(dodges)).min(150);
    let w = i16::try_from(150 * pct / 100).unwrap_or(150);
    let h = i16::try_from(56 * pct / 100).unwrap_or(56);
    Rect::new(60 + 75 - w / 2, 176 + 28 - h / 2, w, h)
}

#[cfg(test)]
mod tests {
    use super::yes_rect;
    use crate::common::testing::{Fake, Rig, CH0};
    use geometry::Rect;

    #[test]
    fn yes_grows_six_percent_per_dodge_up_to_150() {
        assert_eq!(yes_rect(0), Rect::new(60, 176, 150, 56));
        assert_eq!(yes_rect(5).w, 195);
        assert_eq!(yes_rect(200).w, 225);
    }

    #[test]
    fn yes_grows_around_its_centre() {
        let center = |r: Rect| (r.x + r.w / 2, r.y + r.h / 2);
        assert_eq!(center(yes_rect(0)), center(yes_rect(5)));
    }

    #[test]
    fn next_unlocked_follows_found_objects_and_resets_each_chapter() {
        let mut r = Rig::new();
        r.tap(1, 1);
        assert!(!r.m.next_unlocked(&Fake));
        r.tap_rect(CH0[0]);
        assert!(!r.m.next_unlocked(&Fake));
        r.tap_rect(CH0[1]);
        assert!(r.m.next_unlocked(&Fake));
        r.tap_next();
        assert!(!r.m.next_unlocked(&Fake), "next chapter is locked again");
    }

    #[test]
    fn next_is_never_unlocked_outside_the_story() {
        let r = Rig::new();
        assert!(!r.m.next_unlocked(&Fake));
    }
}
