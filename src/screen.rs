//! What the LCD shows: one function per scene, drawn from the [`Model`].
//!
//! Generic over any embedded-graphics target, so the PC tests draw exactly the
//! frames the board draws. The board calls [`Painter::prepare`] once per frame
//! and then [`Painter::draw`] once per band of rows (see `ui::frame::Band`).
//! Every helper skips what lies outside the target, so drawing a band only
//! costs what touches that band.

use core::fmt::Write as _;

use embedded_graphics::geometry::Dimensions;
use embedded_graphics::mono_font::ascii::{FONT_10X20, FONT_5X8, FONT_6X10, FONT_7X13};
use embedded_graphics::mono_font::{MonoFont, MonoTextStyle};
use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::{DrawTarget, Point as GPoint, Primitive, Size};
use embedded_graphics::primitives::{
    Circle, Line as Segment, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, RoundedRectangle,
    Triangle,
};
use embedded_graphics::text::{Baseline, Text};
use embedded_graphics::Drawable;
use ui::frame::Scaled;
use ui::icon::{Icon, Tones};
use ui::mascot::{self, Mood};
use ui::photo::Photo;
use ui::{backdrop, heart as heart_shape, theme};

use crate::event::Effects;
use crate::fonts::{PRESS_START, VT323};
use crate::state::{
    Model, Scene, MAX_DODGES, NEXT_BUTTON, PHOTO_CARD, RESTART_BUTTON, SCREEN_W, SLIDESHOW_CARD,
};
use crate::story::{full_mask, Content, CHAPTERS, ENDING, TRACE};
use crate::trace::{self, DOTS, DOT_AT};
use crate::{Point, Rect};

/// How long a touch ripple grows, in milliseconds.
const RIPPLE_MS: u32 = 450;
/// How long a hint pulses around an object.
const NUDGE_MS: u32 = 900;
/// How long a tapped object pops.
const POP_MS: u32 = 300;
/// Text that types itself out shows one more character this often.
const TYPE_MS: u32 = 28;
/// The celebration message starts typing this long after the heart wipe ends.
const MESSAGE_DELAY_MS: u32 = 300;
/// How long the intro takes to fade in from the background pink.
pub const FADE_MS: u32 = 450;
/// How long the heart wipe takes to open a new scene.
pub const WIPE_MS: u32 = 700;
/// How long a polaroid takes to drop into place.
const DROP_MS: u32 = 350;
/// How long its photo then takes to develop from a blank.
const DEVELOP_MS: u32 = 2_600;

/// Half the sticker's size: it is placed by its middle.
const STICKER_HALF: i16 = 24;

/// Where the sticker on the LCD controller's second layer goes (its top-left
/// corner), or `None` to hide it. It rides just above her finger while she
/// traces the heart, and floats in a lazy figure of eight over the right of
/// the celebration once the heart wipe has opened it.
pub fn sticker_spot(model: &Model, t_ms: u32) -> Option<(i16, i16)> {
    let since = t_ms.wrapping_sub(model.scene_start_ms);
    match model.scene {
        Scene::Trace { .. } if model.traced_ms.is_none() => {
            let pen = model.pen?;
            Some((pen.x - STICKER_HALF, pen.y - 3 * STICKER_HALF))
        }
        Scene::Celebration if since >= WIPE_MS => {
            let x = 340 + ui::sticker::wave(t_ms, 5_200) * 110 / 1_000;
            let y = 140 + ui::sticker::wave(t_ms.wrapping_mul(2), 5_200) * 45 / 1_000;
            Some((to_i16(x) - STICKER_HALF, to_i16(y) - STICKER_HALF))
        }
        _ => None,
    }
}

/// How much of the picture shows, from 0 (only the background pink) to 255,
/// for the LCD controller's layer fade. The intro fades in: at power-on and
/// whenever the story goes back to the beginning. The other scenes open with
/// the heart wipe instead.
pub fn opacity(model: &Model, t_ms: u32) -> u8 {
    if model.scene != Scene::Intro {
        return u8::MAX;
    }
    let since = t_ms.wrapping_sub(model.scene_start_ms);
    if since >= FADE_MS {
        return u8::MAX;
    }
    // Ease out: quick at first, settling gently at the end.
    let left = (FADE_MS - since) * 255 / FADE_MS;
    u8::try_from(255 - left * left / 255).unwrap_or(u8::MAX)
}
/// Text of the intro screen.
const INTRO_TITLE: &str = "For you";
/// Label of the intro button.
const INTRO_BUTTON: &str = "Open";
/// Small line under the intro button.
const INTRO_HINT: &str = "tap to open, or pet me";
/// What the dog's speech bubble says when it wants attention.
const ATTRACT_LINE: &str = "tap me!";
/// The dog plays in cycles this long: jumping first, then a happy bounce.
const ATTRACT_CYCLE_MS: u32 = 5_000;
/// How long the jumping lasts in each cycle.
const ATTRACT_JUMP_MS: u32 = 2_000;
/// Label of the button that goes back to the intro.
const RESTART_LABEL: &str = "Start again";
/// The card under the dog that shows the revealed line, in the bottom-left
/// corner.
const CARD: Rect = Rect::new(12, 211, 266, 51);
/// Space between the card's edge and its text, left and right.
const CARD_PAD_X: i32 = 8;
/// Space above the first line of text in the card.
const CARD_PAD_TOP: i32 = 4;
/// Left column of the intro and the celebration, where the dog sits.
const RIGHT_COLUMN_X: i32 = 200;
/// Colors of a found object's icon: a pink fill under a deep pink outline.
const ICON_TONES: Tones = Tones {
    back: theme::SURFACE,
    fill: theme::ICON_FILL,
    line: theme::ICON,
};

/// The short-lived effects (ripple, hint, pop), remembered with the time they
/// started so they can animate for a while after the event that caused them.
#[derive(Copy, Clone, Debug, Default, PartialEq, Eq)]
pub struct Cues {
    /// Where the last touch landed, and when.
    pub ripple: Option<(Point, u32)>,
    /// The object to pulse as a hint, and when the hint started.
    pub nudge: Option<(u8, u32)>,
    /// The object that was just tapped, and when.
    pub reveal: Option<(u8, u32)>,
}

impl Cues {
    /// No effects.
    pub const fn new() -> Self {
        Self {
            ripple: None,
            nudge: None,
            reveal: None,
        }
    }

    /// Remembers the one-shot effects of `fx`, which happened at `now_ms`.
    pub fn record(&mut self, fx: &Effects, now_ms: u32) {
        if fx.scene_changed {
            self.nudge = None;
            self.reveal = None;
        }
        if let Some(p) = fx.ripple {
            self.ripple = Some((p, now_ms));
        }
        if let Some(i) = fx.nudge {
            self.nudge = Some((i, now_ms));
        }
        if let Some(i) = fx.reveal {
            self.reveal = Some((i, now_ms));
            self.nudge = None;
        }
    }
}

/// Milliseconds since `start`, if an effect started then is still running.
fn running(start: u32, now: u32, length: u32) -> Option<u32> {
    let age = now.wrapping_sub(start);
    (age < length).then_some(age)
}

/// The mood of the dog in each scene.
fn mood(model: &Model, t_ms: u32) -> Mood {
    match model.scene {
        Scene::Intro => match model.attracting_for(t_ms) {
            // Untouched for a while: jump around, then bounce, again and again.
            Some(a) if a % ATTRACT_CYCLE_MS < ATTRACT_JUMP_MS => Mood::Cheer,
            _ => Mood::Happy,
        },
        Scene::Story { chapter, .. } => CHAPTERS
            .get(usize::from(chapter))
            .and_then(|c| Mood::from_name(c.mood))
            .unwrap_or(Mood::Idle),
        // Watching her draw, then in love once the heart is done.
        Scene::Trace { .. } if model.traced_ms.is_some() => Mood::Love,
        Scene::Trace { .. } => Mood::Wow,
        Scene::Question if model.no_gave_up() => Mood::Love,
        Scene::Question if model.dodges > 0 => Mood::Wow,
        Scene::Question => Mood::Shy,
        Scene::Celebration => Mood::Cheer,
    }
}

/// Where the dog sits right now, and how big each sprite pixel is.
fn dog_place(model: &Model) -> (GPoint, u32) {
    let (x, y, scale) = model.dog_spot();
    (GPoint::new(i32::from(x), i32::from(y)), u32::from(scale))
}

/// Each photo of the album slideshow stays this long.
const SLIDE_MS: u32 = 4_000;

/// Draws whole frames. Holds the parts of a frame that are worked out once
/// and then reused for every band.
pub struct Painter {
    model: Model,
    cues: Cues,
    t_ms: u32,
    dog: [u8; mascot::CELLS],
    glyphs: [u8; backdrop::CELLS],
    /// The album's photos, one after another (see `crate::album`), if one
    /// was loaded.
    album: Option<&'static [u8]>,
}

impl Default for Painter {
    fn default() -> Self {
        Self::new()
    }
}

impl Painter {
    /// A painter for the intro at time zero.
    pub const fn new() -> Self {
        Self {
            model: Model::new(0),
            cues: Cues::new(),
            t_ms: 0,
            dog: [mascot::CLEAR; mascot::CELLS],
            glyphs: [0; backdrop::CELLS],
            album: None,
        }
    }

    /// A painter that shows `album` (photos one after another, as
    /// `crate::album` lays them out) as a slideshow on the celebration.
    pub const fn with_album(album: &'static [u8]) -> Self {
        let mut p = Self::new();
        p.album = Some(album);
        p
    }

    /// Photo `i` of the album, if there is one.
    fn album_photo(&self, i: u32) -> Option<Photo> {
        let bytes = crate::album::PHOTO_BYTES as usize;
        let start = usize::try_from(i).ok()? * bytes;
        let px = self.album?.get(start..start + bytes)?;
        Some(Photo::new(to_u16(crate::album::SIDE), px))
    }

    /// Works out the frame for `model` at `t_ms` milliseconds since boot.
    pub fn prepare(&mut self, model: &Model, cues: &Cues, t_ms: u32) {
        self.model = *model;
        self.cues = *cues;
        self.t_ms = t_ms;
        backdrop::render(t_ms, &mut self.glyphs);
        let since = t_ms.wrapping_sub(model.scene_start_ms);
        let pose = match model.petted_for(t_ms) {
            // Petted: heart eyes and a fast wag for a moment.
            Some(petted) => mascot::pose(Mood::Love, petted),
            None => mascot::pose(mood(model, t_ms), since),
        };
        mascot::render(&pose, &mut self.dog);
    }

    /// Draws the prepared frame. Only the part inside the target's bounding
    /// box is drawn, so this can be called once per band.
    pub fn draw<D>(&self, target: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let clip = target.bounding_box();
        target.fill_solid(&clip, theme::BACKGROUND)?;
        self.draw_backdrop(target)?;
        match self.model.scene {
            Scene::Intro => self.intro(target)?,
            Scene::Story {
                chapter,
                found,
                shown,
            } => self.story(target, chapter, found, shown)?,
            Scene::Trace { hit } => self.trace(target, hit)?,
            Scene::Question => self.question(target)?,
            Scene::Celebration => self.celebration(target)?,
        }
        let (at, scale) = dog_place(&self.model);
        self.draw_dog(target, at, scale)?;
        self.pet_hearts(target)?;
        self.speech_bubble(target)?;
        self.ripple(target)?;
        if let Some((center, size)) = self.wipe() {
            heart_shape::fill_outside(target, center, size, theme::TILE)?;
        }
        Ok(())
    }

    /// While a scene going forward opens, the heart it shows through: its
    /// center and size. The heart grows from where the tap that opened the
    /// scene landed (or the middle of the screen), slowly at first and then
    /// quickly, until it covers the whole screen.
    fn wipe(&self) -> Option<(GPoint, i32)> {
        if self.model.scene == Scene::Intro {
            return None;
        }
        let since = self.since();
        if since >= WIPE_MS {
            return None;
        }
        let center = match self.cues.ripple {
            Some((p, at)) if at == self.model.scene_start_ms => {
                GPoint::new(i32::from(p.x), i32::from(p.y))
            }
            _ => GPoint::new(i32::from(SCREEN_W) / 2, 136),
        };
        let full = heart_shape::covering_size(center, i32::from(SCREEN_W), 272);
        let p = to_i32_u(since * 1_000 / WIPE_MS);
        Some((center, full * p * p / 1_000_000))
    }

    fn draw_backdrop<D>(&self, target: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let clip = target.bounding_box();
        for row in 0..backdrop::ROWS {
            let top = to_i32(row) * backdrop::CELL_PX;
            if !touches_rows(&clip, top, backdrop::CELL_PX) {
                continue;
            }
            for col in 0..backdrop::COLS {
                let b = self.glyphs[row * backdrop::COLS + col];
                if b == 0 {
                    continue;
                }
                let n = usize::from(b & 0x0f);
                let Some(ch) = n.checked_sub(1).and_then(|i| backdrop::GLYPHS.get(i)) else {
                    continue;
                };
                let color = theme::backdrop_color(b >> 4);
                let base = GPoint::new(to_i32(col) * backdrop::CELL_PX + 1, top + 1);
                for (y, bits) in backdrop::mark(*ch).iter().enumerate() {
                    for x in 0..5 {
                        if bits & (0b10000 >> x) != 0 {
                            let at = base + GPoint::new(x, to_i32(y));
                            target.fill_solid(&Rectangle::new(at, Size::new(2, 2)), color)?;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn draw_dog<D>(&self, target: &mut D, origin: GPoint, scale: u32) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let clip = target.bounding_box();
        let s = to_i32_u(scale);
        for y in 0..mascot::H {
            let top = origin.y + to_i32(y) * s;
            if !touches_rows(&clip, top, s) {
                continue;
            }
            let row = &self.dog[y * mascot::W..(y + 1) * mascot::W];
            let mut x = 0;
            while x < mascot::W {
                let c = row[x];
                let start = x;
                while x < mascot::W && row[x] == c {
                    x += 1;
                }
                if let Some(color) = theme::mascot_color(c) {
                    let at = GPoint::new(origin.x + to_i32(start) * s, top);
                    let size = Size::new(to_u32(x - start) * scale, scale);
                    target.fill_solid(&Rectangle::new(at, size), color)?;
                }
            }
        }
        Ok(())
    }

    fn intro<D>(&self, target: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let center = (RIGHT_COLUMN_X + i32::from(SCREEN_W)) / 2;
        pixel_text_centered(target, INTRO_TITLE, center, 70, 3, theme::TEXT)?;
        let button = Rect::new(to_i16(center - 70), 140, 140, 48);
        let pulse = to_i16(triangle(self.since(), 1_200, 5));
        pill_shadow(target, button)?;
        ring(target, grow(button, 2 + pulse), 24, theme::TILE, 3)?;
        pill(target, button, theme::ICON, None)?;
        label(target, button, INTRO_BUTTON, theme::ON_ICON)?;
        text_centered(target, INTRO_HINT, center, 204, &FONT_7X13, theme::MUTED)
    }

    fn story<D>(
        &self,
        target: &mut D,
        chapter: u8,
        found: u8,
        shown: Option<u8>,
    ) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let ch = usize::from(chapter);
        let Some(page) = CHAPTERS.get(ch) else {
            return Ok(());
        };

        // Title, wrapped to the left column, then one dot per chapter.
        let mut y = 14;
        for line in wrap(page.title, 15).take(2) {
            pixel_text(target, line, GPoint::new(16, y), 2, theme::TEXT)?;
            y += 20;
        }
        self.trail(target, ch, y + 4)?;

        // The objects, or the chapter's polaroid once they are all found.
        let n = page.hotspots.len();
        if let Some((photo, age)) = page.photo.zip(self.model.photo_for(self.t_ms)) {
            polaroid(target, photo, page.caption, age)?;
        } else {
            for (i, h) in page.hotspots.iter().enumerate() {
                let Ok(idx) = u8::try_from(i) else { break };
                self.object(
                    target,
                    h.rect,
                    h.art,
                    idx,
                    found & (1 << i) != 0,
                    shown == Some(idx),
                )?;
            }
        }

        // The card with the revealed line, or a hint.
        shadow(target, CARD, 10)?;
        rounded(target, CARD, 10, theme::SURFACE, Some((theme::MAUVE, 2)))?;
        let left = i32::from(CARD.x) + CARD_PAD_X;
        let top = i32::from(CARD.y) + CARD_PAD_TOP;
        match shown.and_then(|i| page.hotspots.get(usize::from(i))) {
            Some(h) => {
                // The line types itself out from the moment the object is tapped.
                let shown_chars = match (shown, self.cues.reveal) {
                    (Some(i), Some((j, start))) if i == j => typed(self.t_ms.wrapping_sub(start)),
                    _ => usize::MAX,
                };
                let (font, width, lines, step) = card_font(h.line);
                let typing = Typing {
                    shown: shown_chars,
                    font,
                    scale: 1,
                    color: theme::TEXT,
                    step,
                    t_ms: self.t_ms,
                };
                typing.draw(
                    target,
                    wrap(h.line, width).take(lines),
                    Align::Left(left),
                    top,
                )?;
            }
            None => {
                let left_to_find = n - (found & full_mask(n)).count_ones() as usize;
                let mut hint = Line::new();
                if left_to_find == 0 {
                    let _ = write!(hint, "All found. Tap Next.");
                } else if left_to_find == n {
                    let _ = write!(hint, "Tap the ? tiles to find {n} things.");
                } else {
                    let _ = write!(hint, "{left_to_find} more to find.");
                }
                let middle = i32::from(CARD.y) + (i32::from(CARD.h) - 13) / 2;
                text(
                    target,
                    hint.as_str(),
                    GPoint::new(left, middle),
                    &FONT_7X13,
                    theme::MUTED,
                )?;
            }
        }

        // Next.
        let unlocked = self.model.next_unlocked(&Content);
        shadow(target, NEXT_BUTTON, 12)?;
        if unlocked {
            let pulse = to_i16(triangle(self.since(), 900, 4));
            ring(target, grow(NEXT_BUTTON, 2 + pulse), 14, theme::TILE, 3)?;
            rounded(target, NEXT_BUTTON, 12, theme::ICON, None)?;
            label(target, NEXT_BUTTON, "Next>", theme::ON_ICON)
        } else {
            rounded(target, NEXT_BUTTON, 12, theme::MAUVE, None)?;
            label(target, NEXT_BUTTON, "Next", theme::MUTED)
        }
    }

    /// One dot per chapter: done, current and still to come.
    fn trail<D>(&self, target: &mut D, current: usize, y: i32) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let count = CHAPTERS.len().max(1);
        let step = (240 / to_i32(count)).clamp(6, 14);
        for i in 0..count {
            let x = 16 + to_i32(i) * step;
            let (d, style) = if i == current {
                (8, PrimitiveStyle::with_fill(theme::ICON))
            } else if i < current {
                (6, PrimitiveStyle::with_fill(theme::TILE))
            } else {
                (6, PrimitiveStyle::with_stroke(theme::MAUVE, 2))
            };
            let off = (8 - to_i32_u(d)) / 2;
            Circle::new(GPoint::new(x + off, y + off), d)
                .into_styled(style)
                .draw(target)?;
        }
        Ok(())
    }

    fn object<D>(
        &self,
        target: &mut D,
        rect: Rect,
        art: &Icon,
        idx: u8,
        found: bool,
        shown: bool,
    ) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        // Unfound objects bob gently, each a little out of step with the others.
        let bob = if found {
            0
        } else {
            triangle(self.t_ms + u32::from(idx) * 400, 1_600, 3) - 1
        };
        let mut r = Rect::new(rect.x, rect.y + to_i16(bob), rect.w, rect.h);
        if let Some((i, start)) = self.cues.reveal {
            if let Some(age) = running(start, self.t_ms, POP_MS) {
                if i == idx {
                    r = grow(r, to_i16(to_i32_u((POP_MS - age) / 50)));
                }
            }
        }
        if let Some((i, start)) = self.cues.nudge {
            if let Some(age) = running(start, self.t_ms, NUDGE_MS) {
                if i == idx {
                    let pad = 3 + to_i16(to_i32_u(age / 150 % 2) * 3);
                    ring(target, grow(r, pad), 18, theme::ICON, 3)?;
                }
            }
        }
        // The shadow stays where the tile rests, so a bobbing tile floats.
        shadow(target, rect, 14)?;
        let (fill, border) = match (found, shown) {
            (_, true) => (theme::SURFACE, (theme::ICON, 4)),
            (true, false) => (theme::SURFACE, (theme::MAUVE, 3)),
            (false, false) => (theme::TILE, (theme::ICON, 3)),
        };
        rounded(target, r, 14, fill, Some(border))?;
        let (cx, cy) = center(r);
        if found {
            // The object's own icon, from the story file.
            let half = i32::from(art.size) / 2;
            art.draw(target, GPoint::new(cx - half, cy - half), ICON_TONES)
        } else {
            pixel_text_centered(target, "?", cx, cy - 12, 3, theme::ICON)
        }
    }

    /// The dotted heart she traces. Traced dots turn rose and join up into
    /// the heart's outline; the next dot to trace pulses after a while; once
    /// it is done the whole outline glows and "Perfect!" pops up inside.
    fn trace<D>(&self, target: &mut D, hit: u32) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let done = self.model.traced_for(self.t_ms);
        let center_x = (i32::from(DOT_AT[6].0) + i32::from(DOT_AT[18].0)) / 2;
        let scale = pixel_scale(TRACE.prompt, 2 * (470 - center_x), 2);
        pixel_text_centered(target, TRACE.prompt, center_x, 18, scale, theme::TEXT)?;
        let traced = (hit & trace::ALL).count_ones();
        let mut count = Line::new();
        let _ = write!(count, "{traced} / {DOTS}");
        text_centered(
            target,
            count.as_str(),
            center_x,
            46,
            &FONT_7X13,
            theme::MUTED,
        )?;

        let dot = |i: usize| GPoint::new(i32::from(DOT_AT[i].0), i32::from(DOT_AT[i].1));
        let is_hit = |i: usize| hit & 1 << i != 0;
        let width = if done.is_some() { 7 } else { 5 };
        let line = PrimitiveStyle::with_stroke(theme::HEART, width);
        for i in 0..DOTS {
            let j = (i + 1) % DOTS;
            if done.is_some() || (is_hit(i) && is_hit(j)) {
                Segment::new(dot(i), dot(j))
                    .into_styled(line)
                    .draw(target)?;
            }
        }
        for i in 0..DOTS {
            let (d, style) = if is_hit(i) || done.is_some() {
                (12, PrimitiveStyle::with_fill(theme::HEART))
            } else {
                (10, PrimitiveStyle::with_stroke(theme::ICON, 2))
            };
            let at = dot(i) - GPoint::new(to_i32_u(d) / 2, to_i32_u(d) / 2);
            Circle::new(at, d).into_styled(style).draw(target)?;
        }

        // The hint: the next dot to trace pulses.
        if let Some((i, start)) = self.cues.nudge {
            if let Some(age) = running(start, self.t_ms, NUDGE_MS) {
                if done.is_none() && !is_hit(usize::from(i)) {
                    let d = 18 + to_u32_i(to_i32_u(age / 150 % 2) * 8);
                    let p =
                        dot(usize::from(i) % DOTS) - GPoint::new(to_i32_u(d) / 2, to_i32_u(d) / 2);
                    Circle::new(p, d)
                        .into_styled(PrimitiveStyle::with_stroke(theme::ICON, 3))
                        .draw(target)?;
                }
            }
        }

        if let Some(age) = done {
            // "Perfect!" grows into place inside the finished heart.
            // It pops in: small for a moment, then full size.
            let (scale, y) = if age < 150 { (1, 132) } else { (2, 128) };
            pixel_text_centered(target, "Perfect!", center_x, y, scale, theme::ICON)?;
        } else if self.model.next_unlocked(&Content) {
            shadow(target, NEXT_BUTTON, 12)?;
            rounded(target, NEXT_BUTTON, 12, theme::ICON, None)?;
            label(target, NEXT_BUTTON, "Next>", theme::ON_ICON)?;
        }
        Ok(())
    }

    fn question<D>(&self, target: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        // The question types itself out once the heart wipe has opened.
        let typing = Typing {
            shown: self.since().checked_sub(WIPE_MS).map_or(0, typed),
            font: &PRESS_START,
            scale: 2,
            color: theme::TEXT,
            step: 24,
            t_ms: self.t_ms,
        };
        typing.draw(
            target,
            // 23 characters of 16 pixels end left of the dog in the corner.
            wrap(ENDING.question.en, 23).take(3),
            Align::Left(16),
            24,
        )?;

        let no = self.model.no_rect;
        let label_index = usize::from(self.model.dodges.min(MAX_DODGES));
        let no_label = ENDING.no_labels.get(label_index).copied().unwrap_or("No");
        pill_shadow(target, no)?;
        pill(target, no, theme::SURFACE, Some((theme::TEXT, 3)))?;
        label(target, no, no_label, theme::TEXT)?;

        let yes = self.model.yes_rect();
        let pulse = to_i16(triangle(self.since(), 1_000, 4));
        pill_shadow(target, yes)?;
        ring(target, grow(yes, 2 + pulse), 30, theme::TILE, 3)?;
        pill(target, yes, theme::ICON, None)?;
        label(target, yes, ENDING.yes, theme::ON_ICON)
    }

    fn celebration<D>(&self, target: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        // The album, one photo after another, each developing in turn.
        if self.model.album > 0 {
            let shown = self.since().saturating_sub(WIPE_MS);
            let i = shown / SLIDE_MS % u32::from(self.model.album);
            if let Some(photo) = self.album_photo(i) {
                let mut caption = Line::new();
                let _ = write!(caption, "{} / {}", i + 1, self.model.album);
                polaroid_at(
                    target,
                    SLIDESHOW_CARD,
                    &photo,
                    caption.as_str(),
                    shown % SLIDE_MS,
                )?;
            }
        }

        // Hearts float up the right side, each at its own pace.
        for k in 0..14u32 {
            let h = hash(k.wrapping_mul(0x9e37_79b9) ^ 0x51);
            let period = 2_600 + h % 2_400;
            let phase = (self.t_ms.wrapping_add(h)) % period;
            let x = RIGHT_COLUMN_X + to_i32_u(h / 7 % 260);
            let y = 272 - to_i32_u(phase * 300 / period);
            let color = if k % 3 == 0 {
                theme::TILE
            } else {
                theme::HEART
            };
            heart(target, GPoint::new(x, y), 3, color)?;
        }

        let center = (RIGHT_COLUMN_X + i32::from(SCREEN_W)) / 2;
        let room = i32::from(SCREEN_W) - RIGHT_COLUMN_X - 10;
        let mut y = 60;
        if pixel_width(ENDING.title, 2) <= room {
            pixel_text_centered(target, ENDING.title, center, y, 2, theme::TEXT)?;
            y += 36;
        } else {
            for line in wrap(ENDING.title, usize::try_from(room / 8).unwrap_or(1)).take(2) {
                pixel_text_centered(target, line, center, y, 1, theme::TEXT)?;
                y += 14;
            }
            y += 10;
        }
        // The message types itself out once the title has had a moment.
        let typing = Typing {
            shown: self
                .since()
                .checked_sub(WIPE_MS + MESSAGE_DELAY_MS)
                .map_or(0, typed),
            font: &VT323,
            scale: 1,
            color: theme::ICON,
            step: 22,
            t_ms: self.t_ms,
        };
        typing.draw(
            target,
            wrap(ENDING.message.en, 32).take(3),
            Align::Center(center),
            y,
        )?;

        pill_shadow(target, RESTART_BUTTON)?;
        pill(
            target,
            RESTART_BUTTON,
            theme::SURFACE,
            Some((theme::ICON, 3)),
        )?;
        label(target, RESTART_BUTTON, RESTART_LABEL, theme::ICON)
    }

    /// "tap me!" in a bubble over the dog's head, bobbing, while the dog
    /// plays for attention on an untouched intro.
    fn speech_bubble<D>(&self, target: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let Some(age) = self.model.attracting_for(self.t_ms) else {
            return Ok(());
        };
        let dog = self.model.dog_rect();
        let bob = to_i16(triangle(age, 1_000, 4));
        let w = to_i16(text_width(ATTRACT_LINE, &FONT_7X13)) + 20;
        let bubble = Rect::new(dog.x + dog.w / 2 + 6, 8 + bob, w, 28);
        // The tail points down and left, towards the dog's head.
        let (bx, by) = (i32::from(bubble.x), i32::from(bubble.y + bubble.h));
        let tail = Triangle::new(
            GPoint::new(bx + 10, by - 2),
            GPoint::new(bx + 24, by - 2),
            GPoint::new(bx + 2, by + 10),
        );
        let style = PrimitiveStyleBuilder::new()
            .fill_color(theme::SURFACE)
            .stroke_color(theme::ICON)
            .stroke_width(2)
            .build();
        shadow(target, bubble, 12)?;
        tail.into_styled(style).draw(target)?;
        rounded(target, bubble, 12, theme::SURFACE, Some((theme::ICON, 2)))?;
        // Open the bubble's edge where the tail joins it.
        target.fill_solid(
            &Rectangle::new(GPoint::new(bx + 12, by - 3), Size::new(10, 3)),
            theme::SURFACE,
        )?;
        label(target, bubble, ATTRACT_LINE, theme::ICON)
    }

    /// Small hearts that pop out of the dog's head and float up after a pet,
    /// one after another, spreading out as they rise.
    fn pet_hearts<D>(&self, target: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        const HEARTS: u32 = 6;
        const EACH_MS: u32 = 1_100;
        let Some(age) = self.model.petted_for(self.t_ms) else {
            return Ok(());
        };
        let dog = self.model.dog_rect();
        let (_, _, scale) = self.model.dog_spot();
        let size = u32::from(scale.max(2)) - 1;
        let cx = i32::from(dog.x) + i32::from(dog.w) / 2;
        let head = i32::from(dog.y) + i32::from(dog.h) / 6;
        for k in 0..HEARTS {
            let Some(a) = age.checked_sub(k * 110).filter(|a| *a < EACH_MS) else {
                continue;
            };
            let a = to_i32_u(a);
            // Alternate sides, further out each time, drifting outwards.
            let side = if k % 2 == 0 { -1 } else { 1 };
            let spread = side * (6 + to_i32_u(k) * 5);
            let x = cx + spread + spread * a / 900 - to_i32_u(5 * size) / 2;
            let y = head - a * 70 / 1_000;
            let color = if a < 700 { theme::HEART } else { theme::TILE };
            heart(target, GPoint::new(x, y), size, color)?;
        }
        Ok(())
    }

    fn ripple<D>(&self, target: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let Some((p, start)) = self.cues.ripple else {
            return Ok(());
        };
        let Some(age) = running(start, self.t_ms, RIPPLE_MS) else {
            return Ok(());
        };
        let r = 6 + to_i32_u(age / 15);
        let color = if age < RIPPLE_MS / 2 {
            theme::ICON
        } else {
            theme::TILE
        };
        let at = GPoint::new(i32::from(p.x) - r, i32::from(p.y) - r);
        Circle::new(at, to_u32_i(2 * r))
            .into_styled(PrimitiveStyle::with_stroke(color, 3))
            .draw(target)
    }

    /// Milliseconds since the scene started.
    fn since(&self) -> u32 {
        self.t_ms.wrapping_sub(self.model.scene_start_ms)
    }
}

// ---------- small drawing helpers ----------

fn to_i32(v: usize) -> i32 {
    i32::try_from(v).unwrap_or(i32::MAX)
}

fn to_i32_u(v: u32) -> i32 {
    i32::try_from(v).unwrap_or(i32::MAX)
}

fn to_u32(v: usize) -> u32 {
    u32::try_from(v).unwrap_or(0)
}

fn to_u32_i(v: i32) -> u32 {
    u32::try_from(v).unwrap_or(0)
}

fn to_u16(v: u32) -> u16 {
    u16::try_from(v).unwrap_or(u16::MAX)
}

fn to_i16(v: i32) -> i16 {
    i16::try_from(v).unwrap_or(0)
}

/// True if rows `top .. top + height` meet the clip rectangle.
fn touches_rows(clip: &Rectangle, top: i32, height: i32) -> bool {
    let clip_top = clip.top_left.y;
    let clip_bottom = clip_top + to_i32_u(clip.size.height);
    top < clip_bottom && top + height > clip_top
}

/// True if `area` meets the target's bounding box.
fn visible<D: Dimensions>(target: &D, area: &Rectangle) -> bool {
    !target.bounding_box().intersection(area).is_zero_sized()
}

fn rect(r: Rect) -> Rectangle {
    Rectangle::new(
        GPoint::new(i32::from(r.x), i32::from(r.y)),
        Size::new(to_u32_i(i32::from(r.w)), to_u32_i(i32::from(r.h))),
    )
}

fn center(r: Rect) -> (i32, i32) {
    (
        i32::from(r.x) + i32::from(r.w) / 2,
        i32::from(r.y) + i32::from(r.h) / 2,
    )
}

/// `r` made `pad` pixels bigger on every side.
fn grow(r: Rect, pad: i16) -> Rect {
    Rect::new(r.x - pad, r.y - pad, r.w + 2 * pad, r.h + 2 * pad)
}

/// 0 up to `peak` and back down again, once every `period` milliseconds.
fn triangle(t: u32, period: u32, peak: u32) -> i32 {
    let phase = t % period;
    let half = (period / 2).max(1);
    let up = if phase < half { phase } else { period - phase };
    to_i32_u(up * peak / half)
}

fn hash(mut v: u32) -> u32 {
    v ^= v >> 16;
    v = v.wrapping_mul(0x7feb_352d);
    v ^= v >> 15;
    v = v.wrapping_mul(0x846c_a68b);
    v ^ (v >> 16)
}

/// `area` with room for a stroke of `width` drawn on its edge (strokes are
/// centered on the edge, so half of one sticks out).
fn with_stroke(area: Rectangle, width: u32) -> Rectangle {
    let pad = to_i32_u(width);
    Rectangle::new(
        area.top_left - GPoint::new(pad, pad),
        area.size + Size::new(2 * width, 2 * width),
    )
}

fn rounded<D>(
    target: &mut D,
    r: Rect,
    radius: u32,
    fill: Rgb565,
    border: Option<(Rgb565, u32)>,
) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    let area = rect(r);
    let stroke = border.map_or(0, |(_, width)| width);
    if !visible(target, &with_stroke(area, stroke)) {
        return Ok(());
    }
    let mut style = PrimitiveStyleBuilder::new().fill_color(fill);
    if let Some((color, width)) = border {
        style = style.stroke_color(color).stroke_width(width);
    }
    RoundedRectangle::with_equal_corners(area, Size::new(radius, radius))
        .into_styled(style.build())
        .draw(target)
}

/// How far a polaroid that came out `age` ms ago has developed, in
/// fifteenths: nothing while it drops in, then slowly at first.
fn developed(age: u32) -> u8 {
    let Some(t) = age.checked_sub(DROP_MS) else {
        return 0;
    };
    let p = t.min(DEVELOP_MS) * 1_000 / DEVELOP_MS;
    u8::try_from(15 * p * p / 1_000_000).unwrap_or(15)
}

/// A chapter's photo as a polaroid: a warm white frame with the photo in
/// it, the caption under it and a strip of pink tape over the top. It drops
/// in from above, then the photo develops like an instant photo.
fn polaroid<D>(target: &mut D, photo: &Photo, caption: &str, age: u32) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    polaroid_at(target, PHOTO_CARD, photo, caption, age)
}

/// [`polaroid`] resting at `rest`.
fn polaroid_at<D>(
    target: &mut D,
    rest: Rect,
    photo: &Photo,
    caption: &str,
    age: u32,
) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    let drop = DROP_MS.saturating_sub(age) * 60 / DROP_MS;
    let card = lowered(rest, -to_i16(to_i32_u(drop)));
    shadow(target, card, 4)?;
    rounded(target, card, 4, theme::POLAROID, Some((theme::MAUVE, 1)))?;
    let (x, y) = (i32::from(card.x), i32::from(card.y));
    photo.draw(target, GPoint::new(x + 8, y + 8), developed(age))?;
    let cx = x + i32::from(card.w) / 2;
    if !caption.is_empty() {
        let at = GPoint::new(cx - text_width(caption, &VT323) / 2, y + 8 + 128 + 6);
        font_text(target, caption, at, &VT323, 1, theme::TEXT)?;
    }
    let tape = Rectangle::new(GPoint::new(cx - 24, y - 7), Size::new(48, 15));
    if visible(target, &tape) {
        target.fill_solid(&tape, theme::TILE)?;
    }
    Ok(())
}

/// `r` moved down by `dy` pixels.
fn lowered(r: Rect, dy: i16) -> Rect {
    Rect::new(r.x, r.y + dy, r.w, r.h)
}

/// A soft shadow under a rounded shape `r`: a light layer four pixels down
/// and a darker one two pixels down. Draw it before the shape.
fn shadow<D>(target: &mut D, r: Rect, radius: u32) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    rounded(target, lowered(r, 4), radius, theme::SHADOW_SOFT, None)?;
    rounded(target, lowered(r, 2), radius, theme::SHADOW, None)
}

/// [`shadow`] for a [`pill`].
fn pill_shadow<D>(target: &mut D, r: Rect) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    shadow(target, r, to_u32_i(i32::from(r.h.min(r.w)) / 2))
}

/// A rounded rectangle with fully round ends.
fn pill<D>(
    target: &mut D,
    r: Rect,
    fill: Rgb565,
    border: Option<(Rgb565, u32)>,
) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    let radius = to_u32_i(i32::from(r.h.min(r.w)) / 2);
    rounded(target, r, radius, fill, border)
}

/// The outline of a rounded rectangle.
fn ring<D>(target: &mut D, r: Rect, radius: u32, color: Rgb565, width: u32) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    let area = rect(r);
    if !visible(target, &with_stroke(area, width)) {
        return Ok(());
    }
    let radius = radius.min(to_u32_i(i32::from(r.h.min(r.w)) / 2));
    RoundedRectangle::with_equal_corners(area, Size::new(radius, radius))
        .into_styled(PrimitiveStyle::with_stroke(color, width))
        .draw(target)
}

/// The dog's little heart, each art pixel drawn `scale` pixels wide.
fn heart<D>(target: &mut D, at: GPoint, scale: u32, color: Rgb565) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    const ART: [u8; 4] = [0b01010, 0b11111, 0b01110, 0b00100];
    let s = to_i32_u(scale);
    let area = Rectangle::new(at, Size::new(5 * scale, 4 * scale));
    if !visible(target, &area) {
        return Ok(());
    }
    for (y, bits) in ART.iter().enumerate() {
        for x in 0..5 {
            if bits & (0b10000 >> x) != 0 {
                let p = at + GPoint::new(x * s, to_i32(y) * s);
                target.fill_solid(&Rectangle::new(p, Size::new(scale, scale)), color)?;
            }
        }
    }
    Ok(())
}

fn text<D>(
    target: &mut D,
    s: &str,
    at: GPoint,
    font: &MonoFont<'_>,
    color: Rgb565,
) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    // Bold: every glyph is drawn twice, one pixel apart.
    let style = MonoTextStyle::new(font, color);
    let t = Text::with_baseline(s, at, style, Baseline::Top);
    let area = t.bounding_box();
    let area = Rectangle::new(area.top_left, area.size + Size::new(1, 0));
    if visible(target, &area) {
        t.draw(target)?;
        Text::with_baseline(s, at + GPoint::new(1, 0), style, Baseline::Top).draw(target)?;
    }
    Ok(())
}

fn text_width(s: &str, font: &MonoFont<'_>) -> i32 {
    to_i32(s.chars().count()) * to_i32_u(font.character_size.width + font.character_spacing)
}

fn text_centered<D>(
    target: &mut D,
    s: &str,
    cx: i32,
    y: i32,
    font: &MonoFont<'_>,
    color: Rgb565,
) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    let at = GPoint::new(cx - text_width(s, font) / 2, y);
    text(target, s, at, font, color)
}

/// Press Start 2P's cell size before scaling: 8 x 8.
const PIXEL_CELL: i32 = 8;

/// How wide `s` is in Press Start 2P at `scale`.
fn pixel_width(s: &str, scale: u32) -> i32 {
    to_i32(s.chars().count()) * PIXEL_CELL * to_i32_u(scale)
}

/// The biggest scale, up to `max`, at which `s` fits in `room` pixels in
/// Press Start 2P (1 if none does).
fn pixel_scale(s: &str, room: i32, max: u32) -> u32 {
    (1..=max)
        .rev()
        .find(|&k| pixel_width(s, k) <= room)
        .unwrap_or(1)
}

/// Text in any font, every font pixel a `scale` by `scale` square. The pixel
/// fonts are drawn as they are; the plain fonts are drawn bold.
fn font_text<D>(
    target: &mut D,
    s: &str,
    at: GPoint,
    font: &MonoFont<'_>,
    scale: u32,
    color: Rgb565,
) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    if scale <= 1 && !is_pixel_font(font) {
        return text(target, s, at, font, color);
    }
    let k = scale.max(1);
    let cell = font.character_size * k;
    let width = to_u32_i(text_width(s, font)) * k;
    if !visible(target, &Rectangle::new(at, Size::new(width, cell.height))) {
        return Ok(());
    }
    let style = MonoTextStyle::new(font, color);
    if k == 1 {
        Text::with_baseline(s, at, style, Baseline::Top).draw(target)?;
    } else {
        let mut big = Scaled::new(target, at, k);
        Text::with_baseline(s, GPoint::zero(), style, Baseline::Top).draw(&mut big)?;
    }
    Ok(())
}

/// True for the baked pixel fonts, which are drawn as they are rather than
/// bold. Fonts are constants, so they are told apart by their cell size (no
/// built-in font is 8 x 8 or 8 x 20).
fn is_pixel_font(font: &MonoFont<'_>) -> bool {
    font.character_size == PRESS_START.character_size || font.character_size == VT323.character_size
}

/// Press Start 2P at `scale`, its top-left at `at`.
fn pixel_text<D>(
    target: &mut D,
    s: &str,
    at: GPoint,
    scale: u32,
    color: Rgb565,
) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    font_text(target, s, at, &PRESS_START, scale, color)
}

/// Press Start 2P at `scale`, centered on `cx`, its top at `y`.
fn pixel_text_centered<D>(
    target: &mut D,
    s: &str,
    cx: i32,
    y: i32,
    scale: u32,
    color: Rgb565,
) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    let at = GPoint::new(cx - pixel_width(s, scale) / 2, y);
    pixel_text(target, s, at, scale, color)
}

/// The font for a story line in its card: the biggest of 7 x 13, 6 x 10 and
/// 5 x 8 that fits the whole line. Returns the font, characters per line,
/// most lines and the line height.
fn card_font(line: &str) -> (&'static MonoFont<'static>, usize, usize, i32) {
    let fonts: [(&'static MonoFont<'static>, i32); 4] = [
        (&VT323, 20),
        (&FONT_7X13, 14),
        (&FONT_6X10, 11),
        (&FONT_5X8, 9),
    ];
    let room = usize::try_from(i32::from(CARD.w) - 2 * CARD_PAD_X).unwrap_or(0);
    let height = i32::from(CARD.h) - CARD_PAD_TOP - 2;
    let fit = |(font, step): (&'static MonoFont<'static>, i32)| {
        let cell = usize::try_from(font.character_size.width + font.character_spacing).unwrap_or(1);
        let width = room / cell.max(1);
        let lines = usize::try_from(height / step).unwrap_or(0);
        (font, width, lines, step)
    };
    fonts
        .into_iter()
        .map(fit)
        .find(|(_, width, lines, _)| wrap(line, *width).count() <= *lines)
        .unwrap_or_else(|| fit(fonts[3]))
}

/// Writes `s` centered on a button, in the biggest font that fits.
fn label<D>(target: &mut D, r: Rect, s: &str, color: Rgb565) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    let room = i32::from(r.w) - 16;
    let (cx, cy) = center(r);
    // The pixel font where it fits big enough, else the plain fonts.
    for (scale, min_h) in [(3, 52), (2, 36)] {
        if pixel_width(s, scale) <= room && i32::from(r.h) >= min_h {
            let h = PIXEL_CELL * to_i32_u(scale);
            return pixel_text_centered(target, s, cx, cy - h / 2, scale, color);
        }
    }
    let font = [&FONT_10X20, &FONT_7X13, &FONT_6X10]
        .into_iter()
        .find(|f| text_width(s, f) <= room)
        .unwrap_or(&FONT_6X10);
    let h = to_i32_u(font.character_size.height);
    text_centered(target, s, cx, cy - h / 2, font, color)
}

/// How many characters of a text that started typing `age_ms` ago are showing.
fn typed(age_ms: u32) -> usize {
    usize::try_from(age_ms / TYPE_MS).unwrap_or(usize::MAX)
}

/// The first `n` characters of `s`.
fn first_chars(s: &str, n: usize) -> &str {
    s.char_indices().nth(n).map_or(s, |(i, _)| &s[..i])
}

/// Where the lines of a text go across the screen.
#[derive(Copy, Clone)]
enum Align {
    /// Every line starts at this x.
    Left(i32),
    /// Every line is centered on this x.
    Center(i32),
}

/// Text that types itself out: only the first `shown` characters are drawn,
/// with a blinking cursor after the last one until the whole text is out.
struct Typing<'f> {
    shown: usize,
    font: &'f MonoFont<'f>,
    /// Every font pixel becomes a square this many screen pixels wide.
    scale: u32,
    color: Rgb565,
    /// Distance from one line's top to the next.
    step: i32,
    /// The frame's time, for the cursor's blink.
    t_ms: u32,
}

impl Typing<'_> {
    /// Draws `lines`, the first one's top at `top`. A centered line is placed
    /// by its full width, so it does not slide while it types.
    fn draw<'s, D>(
        &self,
        target: &mut D,
        lines: impl Iterator<Item = &'s str>,
        align: Align,
        top: i32,
    ) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let mut left = self.shown;
        for (k, line) in lines.enumerate() {
            let y = top + to_i32(k) * self.step;
            let scale = to_i32_u(self.scale.max(1));
            let x = match align {
                Align::Left(x) => x,
                Align::Center(cx) => cx - text_width(line, self.font) * scale / 2,
            };
            let count = line.chars().count();
            let part = first_chars(line, left);
            font_text(
                target,
                part,
                GPoint::new(x, y),
                self.font,
                self.scale,
                self.color,
            )?;
            if left <= count {
                // Still typing: a blinking block where the next letter goes.
                if (self.t_ms / 250).is_multiple_of(2) {
                    let cell = self.font.character_size * self.scale.max(1);
                    let at = GPoint::new(x + text_width(part, self.font) * scale, y + 1);
                    let block = Rectangle::new(at, Size::new(cell.width / 2 + 1, cell.height - 2));
                    if visible(target, &block) {
                        target.fill_solid(&block, theme::ICON)?;
                    }
                }
                return Ok(());
            }
            // The space the line break took counts as one character.
            left -= count + 1;
        }
        Ok(())
    }
}

/// Splits `s` into lines of at most `width` characters, breaking at spaces
/// where it can.
pub fn wrap(s: &str, width: usize) -> Wrap<'_> {
    Wrap {
        rest: s,
        width: width.max(1),
    }
}

/// The lines of a wrapped text. See [`wrap`].
pub struct Wrap<'a> {
    rest: &'a str,
    width: usize,
}

impl<'a> Iterator for Wrap<'a> {
    type Item = &'a str;

    fn next(&mut self) -> Option<&'a str> {
        let rest = self.rest.trim_start();
        if rest.is_empty() {
            self.rest = rest;
            return None;
        }
        let mut cut = rest.len();
        let mut last_space = None;
        for (count, (i, ch)) in rest.char_indices().enumerate() {
            if ch == '\n' {
                cut = i;
                break;
            }
            if count == self.width {
                cut = if ch == ' ' {
                    i
                } else {
                    last_space.unwrap_or(i)
                };
                break;
            }
            if ch == ' ' {
                last_space = Some(i);
            }
        }
        let (line, tail) = rest.split_at(cut);
        self.rest = tail;
        Some(line.trim_end())
    }
}

/// A short line of text built without a heap.
struct Line {
    buf: [u8; 48],
    len: usize,
}

impl Line {
    const fn new() -> Self {
        Self {
            buf: [0; 48],
            len: 0,
        }
    }

    fn as_str(&self) -> &str {
        core::str::from_utf8(&self.buf[..self.len]).unwrap_or("")
    }
}

impl core::fmt::Write for Line {
    fn write_str(&mut self, s: &str) -> core::fmt::Result {
        let room = self.buf.len() - self.len;
        let take = s.len().min(room);
        self.buf[self.len..self.len + take].copy_from_slice(&s.as_bytes()[..take]);
        self.len += take;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    extern crate std;

    use std::{vec, vec::Vec};

    use ui::frame::{Band, HEIGHT, WIDTH};

    use super::*;
    use crate::common::testing::Rig;
    use crate::{update, Event, Story, TouchEvent, XorShift32};

    fn full_frame(p: &Painter) -> Vec<u16> {
        let mut buf = vec![0u16; WIDTH * HEIGHT];
        p.draw(&mut Band::new(&mut buf, 0)).unwrap();
        buf
    }

    fn banded_frame(p: &Painter, rows: usize) -> Vec<u16> {
        let mut out = vec![0u16; WIDTH * HEIGHT];
        let mut band = vec![0u16; WIDTH * rows];
        for top in (0..HEIGHT).step_by(rows) {
            p.draw(&mut Band::new(&mut band, top)).unwrap();
            let n = band.len().min(out.len() - top * WIDTH);
            out[top * WIDTH..top * WIDTH + n].copy_from_slice(&band[..n]);
        }
        out
    }

    /// The real story played through `update` to each scene.
    fn scenes() -> Vec<(Model, Cues)> {
        let story = Content;
        let mut rng = XorShift32::new(3);
        let mut m = Model::new(0);
        let mut cues = Cues::new();
        let mut out = vec![(m, cues)];
        let mut send = |m: &mut Model, cues: &mut Cues, ev: Event| {
            let fx = update(m, &story, &mut rng, ev);
            cues.record(&fx, m.now_ms);
        };
        let tap =
            |send: &mut dyn FnMut(&mut Model, &mut Cues, Event),
             m: &mut Model,
             cues: &mut Cues,
             x: i16,
             y: i16| send(m, cues, Event::Touch(TouchEvent::Down(Point::new(x, y))));
        tap(&mut send, &mut m, &mut cues, 240, 136);
        out.push((m, cues));
        let r = story.hotspot_rect(0, 0);
        tap(&mut send, &mut m, &mut cues, r.x + 4, r.y + 4);
        out.push((m, cues));
        for ch in 0..story.chapter_count() {
            for i in 0..story.hotspot_count(ch) {
                let r = story.hotspot_rect(ch, i);
                tap(&mut send, &mut m, &mut cues, r.x + 4, r.y + 4);
            }
            tap(
                &mut send,
                &mut m,
                &mut cues,
                NEXT_BUTTON.x + 4,
                NEXT_BUTTON.y + 4,
            );
        }
        assert_eq!(
            m.scene,
            Scene::Trace { hit: 0 },
            "the heart game comes first"
        );
        out.push((m, cues));
        // Trace the heart: a tap on the first dot, then a drag round the rest.
        let (x, y) = DOT_AT[0];
        tap(&mut send, &mut m, &mut cues, x, y);
        for i in 1..=DOTS {
            let (x, y) = DOT_AT[i % DOTS];
            send(
                &mut m,
                &mut cues,
                Event::Touch(TouchEvent::Move(Point::new(x, y))),
            );
        }
        out.push((m, cues));
        let done = m.now_ms + crate::state::TRACE_ADMIRE_MS;
        send(&mut m, &mut cues, Event::Tick { now_ms: done });
        assert_eq!(m.scene, Scene::Question);
        out.push((m, cues));
        let no = m.no_rect;
        tap(&mut send, &mut m, &mut cues, no.x + 4, no.y + 4);
        out.push((m, cues));
        let yes = m.yes_rect();
        tap(&mut send, &mut m, &mut cues, yes.x + 4, yes.y + 4);
        assert_eq!(m.scene, Scene::Celebration);
        out.push((m, cues));
        out
    }

    fn count(frame: &[u16], color: Rgb565) -> usize {
        let raw = theme::to_raw(color);
        frame.iter().filter(|p| **p == raw).count()
    }

    #[test]
    fn every_scene_draws_the_dog_on_the_pink_background() {
        let mut p = Painter::new();
        for (m, cues) in scenes() {
            for t in [WIPE_MS, 1_000, 2_500, 5_000] {
                p.prepare(&m, &cues, m.now_ms + t);
                let f = full_frame(&p);
                assert!(count(&f, theme::BACKGROUND) > f.len() / 3, "{:?}", m.scene);
                assert!(
                    count(&f, theme::mascot_color(b'W').unwrap()) > 300,
                    "{:?}",
                    m.scene
                );
                assert!(count(&f, theme::TEXT) > 100, "{:?} has text", m.scene);
            }
        }
    }

    #[test]
    fn drawing_in_bands_gives_the_same_frame_as_drawing_it_whole() {
        let mut p = Painter::new();
        for (m, cues) in scenes() {
            p.prepare(&m, &cues, m.now_ms + 120);
            let whole = full_frame(&p);
            for rows in [16, 17, 34] {
                assert!(
                    whole == banded_frame(&p, rows),
                    "{:?}, bands of {rows}",
                    m.scene
                );
            }
        }
    }

    #[test]
    fn the_next_button_shows_whether_it_works() {
        let mut r = Rig::new();
        r.tap(1, 1);
        let mut p = Painter::new();
        let center = (NEXT_BUTTON.x as usize + 50) + (NEXT_BUTTON.y as usize + 4) * WIDTH;
        let opened = r.m.now_ms + WIPE_MS;
        p.prepare(&r.m, &Cues::new(), opened);
        assert_eq!(full_frame(&p)[center], theme::to_raw(theme::MAUVE));
        r.m.skip_ok = true;
        p.prepare(&r.m, &Cues::new(), opened);
        assert_eq!(full_frame(&p)[center], theme::to_raw(theme::ICON));
    }

    #[test]
    fn a_touch_draws_a_ripple_that_fades_out() {
        let m = Model::new(0);
        let mut cues = Cues::new();
        cues.record(
            &Effects {
                ripple: Some(Point::new(440, 30)),
                ..Effects::default()
            },
            0,
        );
        let mut p = Painter::new();
        p.prepare(&m, &Cues::new(), 100);
        let plain = full_frame(&p);
        p.prepare(&m, &cues, 100);
        assert_ne!(full_frame(&p), plain);
        p.prepare(&m, &cues, RIPPLE_MS + 100);
        let faded = full_frame(&p);
        p.prepare(&m, &Cues::new(), RIPPLE_MS + 100);
        assert_eq!(faded, full_frame(&p), "the ripple is gone");
    }

    #[test]
    fn cues_forget_hints_on_a_new_scene() {
        let mut c = Cues::new();
        c.record(
            &Effects {
                nudge: Some(1),
                ..Effects::default()
            },
            5,
        );
        assert_eq!(c.nudge, Some((1, 5)));
        c.record(
            &Effects {
                reveal: Some(0),
                ..Effects::default()
            },
            6,
        );
        assert_eq!((c.nudge, c.reveal), (None, Some((0, 6))));
        c.record(
            &Effects {
                scene_changed: true,
                ..Effects::default()
            },
            7,
        );
        assert_eq!((c.nudge, c.reveal), (None, None));
    }

    #[test]
    fn the_intro_fades_in_and_other_scenes_do_not() {
        let mut r = Rig::new();
        assert_eq!(opacity(&r.m, 0), 0, "power-on starts from the pink");
        let mut last = 0;
        for t in (0..=FADE_MS).step_by(30) {
            let a = opacity(&r.m, t);
            assert!(a >= last, "fades in steadily: {a} after {last} at {t} ms");
            last = a;
        }
        assert_eq!(opacity(&r.m, FADE_MS), 255);
        assert_eq!(opacity(&r.m, FADE_MS * 10), 255);
        r.tap(1, 1);
        assert_eq!(opacity(&r.m, r.m.now_ms), 255, "the story uses the wipe");
    }

    #[test]
    fn a_new_chapter_opens_through_a_growing_heart() {
        let mut r = Rig::new();
        r.tap(1, 1);
        let start = r.m.now_ms;
        let mut p = Painter::new();
        let mut wipe_pixels = |t: u32| {
            p.prepare(&r.m, &Cues::new(), start + t);
            count(&full_frame(&p), theme::TILE)
        };
        let at_start = wipe_pixels(0);
        let half_way = wipe_pixels(WIPE_MS / 2);
        let done = wipe_pixels(WIPE_MS);
        assert_eq!(at_start, WIDTH * HEIGHT, "nothing shows at first");
        assert!(half_way < at_start && half_way > done, "{half_way}");
        assert!(done < WIDTH * HEIGHT / 10, "the wipe is gone: {done}");
    }

    #[test]
    fn a_petted_dog_gets_heart_eyes_and_hearts_float_up() {
        let mut r = Rig::new();
        r.tick(FADE_MS);
        let dog = crate::state::dog_rect(Scene::Intro);
        let mut p = Painter::new();
        let mut hearts = |m: &Model, t: u32| {
            p.prepare(m, &Cues::new(), t);
            count(&full_frame(&p), theme::HEART)
        };
        let before = hearts(&r.m, r.m.now_ms + 400);
        r.tap_rect(dog);
        let petted = hearts(&r.m, r.m.now_ms + 400);
        assert!(
            petted > before + 50,
            "heart eyes and hearts: {before} -> {petted}"
        );
        let after = hearts(&r.m, r.m.now_ms + crate::state::PET_MS + 400);
        assert_eq!(after, before, "back to normal once the love wears off");
    }

    #[test]
    fn an_untouched_intro_gets_the_dog_playing_for_attention() {
        use crate::state::ATTRACT_AFTER_MS;
        let m = Model::new(0);
        let mut p = Painter::new();
        // The bubble's text is the icon pink, which the intro otherwise only
        // uses for the Open button on the right.
        let mut bubble = |t: u32| {
            p.prepare(&m, &Cues::new(), t);
            let f = full_frame(&p);
            let raw = theme::to_raw(theme::ICON);
            (0..60)
                .flat_map(|y| (100..300).map(move |x| y * WIDTH + x))
                .filter(|&i| f[i] == raw)
                .count()
        };
        assert_eq!(bubble(ATTRACT_AFTER_MS - 1), 0, "not yet");
        assert!(bubble(ATTRACT_AFTER_MS + 100) > 100, "tap me!");
        assert_eq!(m.attracting_for(ATTRACT_AFTER_MS + 100), Some(100));
        assert_eq!(mood(&m, ATTRACT_AFTER_MS + 100), Mood::Cheer, "jumping");
        assert_eq!(
            mood(&m, ATTRACT_AFTER_MS + 3_000),
            Mood::Happy,
            "then bouncing"
        );
    }

    #[test]
    fn a_touch_stops_the_attention_seeking() {
        use crate::state::ATTRACT_AFTER_MS;
        let mut r = Rig::new();
        r.tick(ATTRACT_AFTER_MS + 500);
        assert!(r.m.attracting_for(r.m.now_ms).is_some());
        r.tap_rect(crate::state::dog_rect(Scene::Intro));
        assert_eq!(r.m.attracting_for(r.m.now_ms), None, "petting counts");
        r.tap(300, 30);
        assert_eq!(
            r.m.attracting_for(r.m.now_ms + ATTRACT_AFTER_MS),
            None,
            "not in the story"
        );
    }

    #[test]
    fn the_sticker_rides_above_the_finger_while_tracing() {
        let mut r = Rig::with_trace();
        r.play_to_question();
        let t = r.m.now_ms;
        assert_eq!(sticker_spot(&r.m, t), None, "no finger yet");
        r.tap(300, 200);
        assert_eq!(sticker_spot(&r.m, t), Some((276, 128)));
        r.drag(320, 180);
        assert_eq!(sticker_spot(&r.m, t), Some((296, 108)), "it follows");
        r.send(Event::Touch(TouchEvent::Up(Point::new(320, 180))));
        assert_eq!(sticker_spot(&r.m, t), None, "gone when the finger lifts");
    }

    #[test]
    fn the_sticker_floats_over_the_celebration_and_nowhere_else() {
        let mut r = Rig::new();
        assert_eq!(sticker_spot(&r.m, 5_000), None, "not on the intro");
        r.tap(1, 1);
        assert_eq!(sticker_spot(&r.m, 5_000), None, "not in the story");
        r.play_to_question_from_story();
        let yes = r.m.yes_rect();
        r.tap_rect(yes);
        let t = r.m.now_ms;
        assert_eq!(sticker_spot(&r.m, t), None, "not under the heart wipe");
        let a = sticker_spot(&r.m, t + WIPE_MS).unwrap();
        let b = sticker_spot(&r.m, t + WIPE_MS + 700).unwrap();
        assert_ne!(a, b, "it moves");
        for dt in (0..10_400).step_by(100) {
            let (x, y) = sticker_spot(&r.m, t + WIPE_MS + dt).unwrap();
            assert!(
                (200..=440).contains(&x) && (60..=200).contains(&y),
                "({x}, {y})"
            );
        }
    }

    #[test]
    fn finding_everything_brings_out_the_chapter_photo_which_can_be_put_aside() {
        let story = Content;
        let ch = (0..story.chapter_count())
            .find(|&c| story.has_photo(c))
            .expect("the default story has a photo");
        let mut rng = XorShift32::new(5);
        let mut m = Model::new(0);
        let mut tap = |m: &mut Model, x: i16, y: i16| {
            update(
                m,
                &story,
                &mut rng,
                Event::Touch(TouchEvent::Down(Point::new(x, y))),
            )
        };
        tap(&mut m, 300, 30);
        for c in 0..ch {
            for i in 0..story.hotspot_count(c) {
                let r = story.hotspot_rect(c, i);
                tap(&mut m, r.x + 4, r.y + 4);
            }
            tap(&mut m, NEXT_BUTTON.x + 4, NEXT_BUTTON.y + 4);
        }
        for i in 0..story.hotspot_count(ch) {
            assert_eq!(m.photo_for(m.now_ms), None, "not before the last object");
            let r = story.hotspot_rect(ch, i);
            tap(&mut m, r.x + 4, r.y + 4);
        }
        assert_eq!(m.photo_for(m.now_ms), Some(0), "out it comes");

        let mut p = Painter::new();
        let mut white = |m: &Model, t: u32| {
            p.prepare(m, &Cues::new(), t);
            let f = full_frame(&p);
            let raw = theme::to_raw(theme::POLAROID);
            let (x0, y0) = (PHOTO_CARD.x as usize, PHOTO_CARD.y as usize);
            (y0..y0 + PHOTO_CARD.h as usize)
                .flat_map(|y| (x0..x0 + PHOTO_CARD.w as usize).map(move |x| y * WIDTH + x))
                .filter(|&i| f[i] == raw)
                .count()
        };
        let t = m.now_ms + WIPE_MS + DROP_MS + DEVELOP_MS;
        assert!(white(&m, t) > 3_000, "the frame is up");
        tap(&mut m, PHOTO_CARD.x + 20, PHOTO_CARD.y + 20);
        assert_eq!(m.photo_for(m.now_ms), None, "put aside");
        assert_eq!(white(&m, t), 0, "and the objects are back");
    }

    #[test]
    fn a_polaroid_develops_slowly_then_all_at_once() {
        assert_eq!(developed(0), 0);
        assert_eq!(developed(DROP_MS), 0, "blank as it lands");
        let half = developed(DROP_MS + DEVELOP_MS / 2);
        assert!((2..=5).contains(&half), "{half}");
        assert_eq!(developed(DROP_MS + DEVELOP_MS), 15);
        assert_eq!(developed(u32::MAX / 2), 15);
    }

    #[test]
    fn the_celebration_shows_the_album_as_a_slideshow() {
        static ALBUM: [u8; 2 * crate::album::PHOTO_BYTES as usize] =
            [0x55; 2 * crate::album::PHOTO_BYTES as usize];
        let mut r = Rig::new();
        r.play_to_question();
        let yes = r.m.yes_rect();
        r.tap_rect(yes);
        let t = r.m.now_ms + WIPE_MS + SLIDE_MS - 100;
        let frame_white = |p: &mut Painter, m: &Model| {
            p.prepare(m, &Cues::new(), t);
            let f = full_frame(p);
            let raw = theme::to_raw(theme::POLAROID);
            let (x0, y0) = (SLIDESHOW_CARD.x as usize, SLIDESHOW_CARD.y as usize);
            (y0..y0 + SLIDESHOW_CARD.h as usize)
                .flat_map(|y| (x0..x0 + SLIDESHOW_CARD.w as usize).map(move |x| y * WIDTH + x))
                .filter(|&i| f[i] == raw)
                .count()
        };
        assert_eq!(
            frame_white(&mut Painter::new(), &r.m),
            0,
            "no album, no slideshow"
        );
        let before = r.m.dog_rect();
        r.m.album = 2;
        assert!(frame_white(&mut Painter::with_album(&ALBUM), &r.m) > 3_000);
        assert_ne!(r.m.dog_rect(), before, "the dog makes room");
        assert!(r.m.dog_rect().w < before.w, "and sits smaller");
    }

    #[test]
    fn the_intro_has_no_wipe() {
        let m = Model::new(0);
        let mut p = Painter::new();
        p.prepare(&m, &Cues::new(), 10);
        assert!(p.wipe().is_none());
    }

    #[test]
    fn text_types_itself_out_one_character_at_a_time() {
        assert_eq!(typed(0), 0);
        assert_eq!(typed(TYPE_MS - 1), 0);
        assert_eq!(typed(TYPE_MS * 5), 5);
        assert_eq!(first_chars("hello", 0), "");
        assert_eq!(first_chars("hello", 2), "he");
        assert_eq!(first_chars("hello", 99), "hello");
        assert_eq!(
            first_chars("héllo", 2),
            "hé",
            "counts characters, not bytes"
        );
    }

    #[test]
    fn the_question_appears_letter_by_letter() {
        let mut r = Rig::new();
        r.play_to_question();
        let start = r.m.now_ms + WIPE_MS;
        let mut p = Painter::new();
        // Only the question's own rows, left of the dog (whose outline is
        // the text color too).
        let mut text_at = |t: u32| {
            p.prepare(&r.m, &Cues::new(), start + t);
            let f = full_frame(&p);
            let raw = theme::to_raw(theme::TEXT);
            (18..96)
                .flat_map(|y| (0..380).map(move |x| y * WIDTH + x))
                .filter(|&i| f[i] == raw)
                .count()
        };
        let early = text_at(TYPE_MS * 3);
        let later = text_at(TYPE_MS * 12);
        let done = text_at(TYPE_MS * 200);
        assert!(early < later && later < done, "{early} < {later} < {done}");
        assert_eq!(done, text_at(TYPE_MS * 400), "finished text stays put");
    }

    #[test]
    fn wrap_breaks_at_spaces_and_never_runs_long() {
        let lines: Vec<&str> = wrap("Tap an object to read its line. Find them all.", 16).collect();
        assert_eq!(
            lines,
            ["Tap an object to", "read its line.", "Find them all."]
        );
        let lines: Vec<&str> = wrap("abcdefghij", 4).collect();
        assert_eq!(lines, ["abcd", "efgh", "ij"]);
        assert_eq!(wrap("  ", 5).count(), 0);
        assert_eq!(wrap("one\ntwo", 20).collect::<Vec<_>>(), ["one", "two"]);
        for line in wrap(CHAPTERS[0].hotspots[0].line, 45) {
            assert!(line.chars().count() <= 45);
        }
    }

    #[test]
    fn every_story_line_fits_its_card() {
        for c in &CHAPTERS {
            for h in c.hotspots {
                let (_, width, lines, _) = card_font(h.line);
                assert!(wrap(h.line, width).count() <= lines, "{}", h.line);
            }
        }
        // Lines up to the 160 characters a story may have still fit, even
        // long words that wrap badly and one word that cannot wrap at all.
        for longest in [
            "abcdefghijklmnopqrstuvw ".repeat(6),
            "a".repeat(160),
            "word ".repeat(32),
        ] {
            let (_, width, lines, step) = card_font(&longest);
            assert!(wrap(&longest, width).count() <= lines, "{longest}");
            assert!(CARD_PAD_TOP + to_i32(lines) * step <= i32::from(CARD.h));
        }
        // Short lines get the tall VT323 pixel font, long ones a plain font.
        let vt323 = |f: &MonoFont<'_>| f.character_size == VT323.character_size;
        assert!(vt323(card_font("Add a memory here.").0));
        assert!(!vt323(card_font(&"word ".repeat(32)).0));
        assert!(is_pixel_font(&VT323) && is_pixel_font(&PRESS_START));
        assert!(!is_pixel_font(&FONT_7X13) && !is_pixel_font(&FONT_10X20));
    }
}
