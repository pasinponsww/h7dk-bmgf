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

#[cfg(test)]
mod tests {
    use super::*;

    struct Lcg(u32);

    impl Lcg {
        fn next(&mut self) -> u32 {
            self.0 = self.0.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            self.0
        }
    }

    const AREA: Rect = Rect::new(10, 104, 460, 158);

    fn placement(avoid: Rect) -> Placement {
        Placement {
            area: AREA,
            avoid,
            pad: 24,
            tries: 64,
        }
    }

    fn inside(area: Rect, r: Rect) -> bool {
        r.x >= area.x
            && r.y >= area.y
            && r.x + r.w <= area.x + area.w
            && r.y + r.h <= area.y + area.h
    }

    #[test]
    fn spots_stay_inside_the_area_and_clear_of_the_avoided_rect() {
        let mut rng = Lcg(1234);
        let mut old = Rect::new(300, 110, 96, 44);
        for round in 0..2000u32 {
            let avoid = if round % 2 == 0 {
                Rect::new(60, 176, 150, 56)
            } else {
                Rect::new(38, 168, 195, 72)
            };
            let next = farthest_free_spot(old, &placement(avoid), &mut || rng.next());
            assert!(inside(AREA, next), "{next:?} left the area");
            assert!(!next.overlaps(&avoid, 24), "{next:?} touches {avoid:?}");
            old = next;
        }
    }

    #[test]
    fn every_move_is_visible_and_usually_a_big_one() {
        let avoid = Rect::new(60, 176, 150, 56);
        let mut rng = Lcg(99);
        let mut old = Rect::new(300, 110, 96, 44);
        let mut total: i64 = 0;
        let rounds = 2000;
        for _ in 0..rounds {
            let next = farthest_free_spot(old, &placement(avoid), &mut || rng.next());
            let (dx, dy) = (i32::from(next.x - old.x), i32::from(next.y - old.y));
            let d2 = dx * dx + dy * dy;
            assert!(d2 > 60 * 60, "moved only {} px", (d2 as f32).sqrt());
            total += i64::from(d2);
            old = next;
        }
        assert!(
            total / i64::from(rounds) > 130 * 130,
            "average move too small"
        );
    }

    #[test]
    fn works_when_the_old_spot_is_cramped() {
        let avoid = Rect::new(38, 168, 195, 72);
        let mut rng = Lcg(5);
        let old = Rect::new(274, 109, 96, 44);
        for _ in 0..500 {
            let next = farthest_free_spot(old, &placement(avoid), &mut || rng.next());
            assert!(!next.overlaps(&avoid, 24));
        }
    }

    #[test]
    fn falls_back_to_the_far_side_when_nothing_is_free() {
        let avoid = Rect::new(0, 0, 480, 272);
        let old = Rect::new(300, 110, 96, 44);
        let mut rng = Lcg(3);
        let spot = farthest_free_spot(old, &placement(avoid), &mut || rng.next());
        assert!(inside(AREA, spot), "{spot:?}");
    }

    #[test]
    fn a_rectangle_as_big_as_the_area_cannot_move() {
        let old = Rect::new(10, 104, 460, 158);
        let mut rng = Lcg(8);
        let spot = farthest_free_spot(old, &placement(Rect::new(0, 0, 1, 1)), &mut || rng.next());
        assert_eq!(spot, old);
    }
}
