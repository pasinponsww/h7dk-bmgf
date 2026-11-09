//! Choosing a free spot for a rectangle.

use crate::Rect;

/// What a new spot has to satisfy.
#[derive(Copy, Clone, Debug)]
pub struct Placement {
    /// The new rectangle must fit entirely inside this area.
    pub area: Rect,
    /// The new rectangle must stay clear of this one.
    pub avoid: Rect,
    /// Extra gap to keep around `avoid`, in pixels.
    pub pad: i16,
    /// How many random spots to try. More tries give a farther jump.
    pub tries: usize,
}

/// Chooses a new rectangle that is as far as possible from the old one
/// Logic that makes sure the YES/NO buttons don't overlap the last one, and that the new button is inside the area
pub fn farthest_free_spot(old: Rect, p: &Placement, rand: &mut impl FnMut() -> u32) -> Rect {
    let span_x = span(p.area.w, old.w);
    let span_y = span(p.area.h, old.h);

    let mut best: Option<(i32, Rect)> = None;
    for _ in 0..p.tries {
        let x = p.area.x + offset(rand(), span_x);
        let y = p.area.y + offset(rand(), span_y);
        let cand = Rect::new(x, y, old.w, old.h);
        if cand.overlaps(&p.avoid, p.pad) {
            continue;
        }
        let dx = i32::from(cand.x - old.x);
        let dy = i32::from(cand.y - old.y);
        let d = dx * dx + dy * dy;
        if best.is_none_or(|(bd, _)| d > bd) {
            best = Some((d, cand));
        }
    }
    if let Some((_, r)) = best {
        return r;
    }

    // Every sample landed on the avoided rectangle. Use the side of the area
    // that is farthest from it.
    let area_mid = i32::from(p.area.x) + i32::from(p.area.w) / 2;
    let avoid_mid = i32::from(p.avoid.x) + i32::from(p.avoid.w) / 2;
    let x = if avoid_mid < area_mid {
        p.area.x + p.area.w - old.w
    } else {
        p.area.x
    };
    Rect::new(x, p.area.y + offset(rand(), span_y), old.w, old.h)
}

/// How many positions fit: the area's size minus the rectangle's, plus one.
fn span(area: i16, size: i16) -> u32 {
    u32::try_from(i32::from(area) - i32::from(size) + 1)
        .unwrap_or(1)
        .max(1)
}

fn offset(random: u32, span: u32) -> i16 {
    i16::try_from((random >> 8) % span).unwrap_or(0)
}
