//! The application state and its tuning constants. This is what RTIC would
//! call the shared resources: every task reads and writes it.

use geometry::Rect;

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
/// Where the NO button starts on the question screen.
pub const INITIAL_NO: Rect = Rect::new(300, 110, 96, 44);
/// The part of the screen the NO button may jump around in. It stays below
/// the question text and away from the edges.
pub const NO_AREA: Rect = Rect::new(10, 104, 460, 158);
/// Gap kept between the NO and YES buttons when NO jumps.
pub const YES_PAD: i16 = 24;
/// How many random spots NO tries on each jump.
pub const JUMP_TRIES: usize = 64;

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
        if scene == Scene::Question {
            self.dodges = 0;
            self.no_rect = INITIAL_NO;
        }
    }

    /// Where the YES button is. It grows 6% per dodge, up to 150%.
    pub fn yes_rect(&self) -> Rect {
        yes_rect(self.dodges)
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
