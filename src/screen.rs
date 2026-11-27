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
    Circle, PrimitiveStyle, PrimitiveStyleBuilder, Rectangle, RoundedRectangle,
};
use embedded_graphics::text::{Baseline, Text};
use embedded_graphics::Drawable;
use ui::frame::Scaled;
use ui::icon::{Icon, Tones};
use ui::mascot::{self, Mood};
use ui::{backdrop, theme};

use crate::event::Effects;
use crate::state::{Model, Scene, MAX_DODGES, NEXT_BUTTON, RESTART_BUTTON, SCREEN_W};
use crate::story::{full_mask, Content, CHAPTERS, ENDING};
use crate::{Point, Rect};

/// How long a touch ripple grows, in milliseconds.
const RIPPLE_MS: u32 = 450;
/// How long a hint pulses around an object.
const NUDGE_MS: u32 = 900;
/// How long a tapped object pops.
const POP_MS: u32 = 300;
/// Text of the intro screen.
const INTRO_TITLE: &str = "For you";
/// Label of the intro button.
const INTRO_BUTTON: &str = "Open";
/// Small line under the intro button.
const INTRO_HINT: &str = "tap anywhere";
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
fn mood(model: &Model) -> Mood {
    match model.scene {
        Scene::Intro => Mood::Happy,
        Scene::Story { chapter, .. } => CHAPTERS
            .get(usize::from(chapter))
            .and_then(|c| Mood::from_name(c.mood))
            .unwrap_or(Mood::Idle),
        Scene::Question if model.no_gave_up() => Mood::Love,
        Scene::Question if model.dodges > 0 => Mood::Wow,
        Scene::Question => Mood::Shy,
        Scene::Celebration => Mood::Cheer,
    }
}

/// Where the dog sits in each scene, and how big each sprite pixel is.
fn dog_place(scene: Scene) -> (GPoint, u32) {
    match scene {
        Scene::Intro => (GPoint::new(40, 56), 4),
        Scene::Story { .. } => (GPoint::new(74, 72), 3),
        Scene::Question => (GPoint::new(396, 14), 2),
        Scene::Celebration => (GPoint::new(36, 56), 4),
    }
}

/// Draws whole frames. Holds the parts of a frame that are worked out once
/// and then reused for every band.
pub struct Painter {
    model: Model,
    cues: Cues,
    t_ms: u32,
    dog: [u8; mascot::CELLS],
    glyphs: [u8; backdrop::CELLS],
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
        }
    }

    /// Works out the frame for `model` at `t_ms` milliseconds since boot.
    pub fn prepare(&mut self, model: &Model, cues: &Cues, t_ms: u32) {
        self.model = *model;
        self.cues = *cues;
        self.t_ms = t_ms;
        backdrop::render(t_ms, &mut self.glyphs);
        let since = t_ms.wrapping_sub(model.scene_start_ms);
        mascot::render(&mascot::pose(mood(model), since), &mut self.dog);
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
            Scene::Question => self.question(target)?,
            Scene::Celebration => self.celebration(target)?,
        }
        let (at, scale) = dog_place(self.model.scene);
        self.draw_dog(target, at, scale)?;
        self.ripple(target)
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
        big_text_centered(target, INTRO_TITLE, center, 64, theme::TEXT, 2)?;
        let button = Rect::new(to_i16(center - 70), 140, 140, 48);
        let pulse = to_i16(triangle(self.since(), 1_200, 5));
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
        let mut y = 12;
        for line in wrap(page.title, 24).take(2) {
            text(target, line, GPoint::new(16, y), &FONT_10X20, theme::TEXT)?;
            y += 22;
        }
        self.trail(target, ch, y + 4)?;

        // The objects.
        let n = page.hotspots.len();
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

        // The card with the revealed line, or a hint.
        rounded(target, CARD, 10, theme::SURFACE, Some((theme::MAUVE, 2)))?;
        let left = i32::from(CARD.x) + CARD_PAD_X;
        let top = i32::from(CARD.y) + CARD_PAD_TOP;
        match shown.and_then(|i| page.hotspots.get(usize::from(i))) {
            Some(h) => {
                let (font, width, lines, step) = card_font(h.line);
                for (k, line) in wrap(h.line, width).take(lines).enumerate() {
                    let at = GPoint::new(left, top + to_i32(k) * step);
                    text(target, line, at, font, theme::TEXT)?;
                }
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
        if unlocked {
            let pulse = to_i16(triangle(self.since(), 900, 4));
            ring(target, grow(NEXT_BUTTON, 2 + pulse), 14, theme::TILE, 3)?;
            rounded(target, NEXT_BUTTON, 12, theme::ICON, None)?;
            label(target, NEXT_BUTTON, "Next >", theme::ON_ICON)
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
            big_text_centered(target, "?", cx, cy - 20, theme::ICON, 2)
        }
    }

    fn question<D>(&self, target: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
        let mut y = 20;
        for line in wrap(ENDING.question.en, 36).take(3) {
            text(target, line, GPoint::new(16, y), &FONT_10X20, theme::TEXT)?;
            y += 24;
        }

        let no = self.model.no_rect;
        let label_index = usize::from(self.model.dodges.min(MAX_DODGES));
        let no_label = ENDING.no_labels.get(label_index).copied().unwrap_or("No");
        pill(target, no, theme::SURFACE, Some((theme::TEXT, 3)))?;
        label(target, no, no_label, theme::TEXT)?;

        let yes = self.model.yes_rect();
        let pulse = to_i16(triangle(self.since(), 1_000, 4));
        ring(target, grow(yes, 2 + pulse), 30, theme::TILE, 3)?;
        pill(target, yes, theme::ICON, None)?;
        label(target, yes, ENDING.yes, theme::ON_ICON)
    }

    fn celebration<D>(&self, target: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = Rgb565>,
    {
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
        if to_i32(ENDING.title.chars().count()) * 20 <= room {
            big_text_centered(target, ENDING.title, center, y, theme::TEXT, 2)?;
            y += 52;
        } else {
            for line in wrap(ENDING.title, 26).take(2) {
                text_centered(target, line, center, y, &FONT_10X20, theme::TEXT)?;
                y += 24;
            }
            y += 8;
        }
        for line in wrap(ENDING.message.en, 26).take(3) {
            text_centered(target, line, center, y, &FONT_10X20, theme::ICON)?;
            y += 24;
        }

        pill(
            target,
            RESTART_BUTTON,
            theme::SURFACE,
            Some((theme::ICON, 3)),
        )?;
        label(target, RESTART_BUTTON, RESTART_LABEL, theme::ICON)
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

/// The 10 x 20 font blown up `scale` times, centered on `cx`, top at `y`.
fn big_text_centered<D>(
    target: &mut D,
    s: &str,
    cx: i32,
    y: i32,
    color: Rgb565,
    scale: u32,
) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    let w = text_width(s, &FONT_10X20) * to_i32_u(scale);
    let at = GPoint::new(cx - w / 2, y);
    let area = Rectangle::new(at, Size::new(to_u32_i(w) + scale, 20 * scale));
    if !visible(target, &area) {
        return Ok(());
    }
    // Bold: drawn twice, one font pixel apart.
    let mut big = Scaled::new(target, at, scale);
    let style = MonoTextStyle::new(&FONT_10X20, color);
    for dx in [0, 1] {
        Text::with_baseline(s, GPoint::new(dx, 0), style, Baseline::Top).draw(&mut big)?;
    }
    Ok(())
}

/// The font for a story line in its card: the biggest of 7 x 13, 6 x 10 and
/// 5 x 8 that fits the whole line. Returns the font, characters per line,
/// most lines and the line height.
fn card_font(line: &str) -> (&'static MonoFont<'static>, usize, usize, i32) {
    let fonts: [(&'static MonoFont<'static>, i32); 3] =
        [(&FONT_7X13, 14), (&FONT_6X10, 11), (&FONT_5X8, 9)];
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
        .unwrap_or_else(|| fit(fonts[2]))
}

/// Writes `s` centered on a button, in the biggest font that fits.
fn label<D>(target: &mut D, r: Rect, s: &str, color: Rgb565) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    let room = i32::from(r.w) - 16;
    let (cx, cy) = center(r);
    if text_width(s, &FONT_10X20) * 2 <= room && i32::from(r.h) >= 52 {
        return big_text_centered(target, s, cx, cy - 20, color, 2);
    }
    let font = [&FONT_10X20, &FONT_7X13, &FONT_6X10]
        .into_iter()
        .find(|f| text_width(s, f) <= room)
        .unwrap_or(&FONT_6X10);
    let h = to_i32_u(font.character_size.height);
    text_centered(target, s, cx, cy - h / 2, font, color)
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
        let mut tap = |m: &mut Model, cues: &mut Cues, x: i16, y: i16| {
            let fx = update(
                m,
                &story,
                &mut rng,
                Event::Touch(TouchEvent::Down(Point::new(x, y))),
            );
            cues.record(&fx, m.now_ms);
        };
        tap(&mut m, &mut cues, 240, 136);
        out.push((m, cues));
        let r = story.hotspot_rect(0, 0);
        tap(&mut m, &mut cues, r.x + 4, r.y + 4);
        out.push((m, cues));
        for ch in 0..story.chapter_count() {
            for i in 0..story.hotspot_count(ch) {
                let r = story.hotspot_rect(ch, i);
                tap(&mut m, &mut cues, r.x + 4, r.y + 4);
            }
            tap(&mut m, &mut cues, NEXT_BUTTON.x + 4, NEXT_BUTTON.y + 4);
        }
        assert_eq!(m.scene, Scene::Question);
        out.push((m, cues));
        let no = m.no_rect;
        tap(&mut m, &mut cues, no.x + 4, no.y + 4);
        out.push((m, cues));
        let yes = m.yes_rect();
        tap(&mut m, &mut cues, yes.x + 4, yes.y + 4);
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
            for t in [0, 250, 1_000, 5_000] {
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
        p.prepare(&r.m, &Cues::new(), r.m.now_ms);
        assert_eq!(full_frame(&p)[center], theme::to_raw(theme::MAUVE));
        r.m.skip_ok = true;
        p.prepare(&r.m, &Cues::new(), r.m.now_ms);
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
        // Short lines keep the big font.
        assert_eq!(card_font("Add a memory here.").0.character_size.width, 7);
    }
}
