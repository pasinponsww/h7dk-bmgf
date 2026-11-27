//! The trace-a-heart game before the big question: a dotted heart, and a
//! finger that goes round it.
//!
//! The dots sit evenly along the classic heart curve
//! (x = 16 sin³t, y = 13 cos t - 5 cos 2t - 2 cos 3t - cos 4t), scaled 5.6
//! times and centered on the right of the screen. A dot counts as traced when
//! the finger passes within [`HIT_RADIUS`] of it. The path between two
//! readings of the touch panel counts too, so a quick finger skips nothing.

use geometry::Point;

/// How many dots go round the heart.
pub const DOTS: usize = 24;
/// The dots, clockwise from the dip at the top.
pub const DOT_AT: [(i16, i16); DOTS] = [
    (300, 120),
    (309, 98),
    (328, 84),
    (352, 82),
    (373, 92),
    (387, 111),
    (389, 134),
    (379, 156),
    (363, 174),
    (346, 190),
    (328, 205),
    (311, 222),
    (300, 243),
    (289, 222),
    (272, 205),
    (254, 190),
    (237, 174),
    (221, 156),
    (211, 134),
    (213, 111),
    (227, 92),
    (248, 82),
    (272, 84),
    (291, 98),
];
/// A finger this close to a dot traces it, in pixels.
pub const HIT_RADIUS: i32 = 20;
/// Traced dots needed to finish: nearly all, so a wobble or two is fine.
pub const NEEDED: u32 = 21;
/// Every dot traced.
pub const ALL: u32 = (1 << DOTS) - 1;

/// Squared distance from `p` to the segment from `a` to `b`.
fn dist_sq_to_segment(p: (i32, i32), a: (i32, i32), b: (i32, i32)) -> i64 {
    let (px, py) = (i64::from(p.0), i64::from(p.1));
    let (ax, ay) = (i64::from(a.0), i64::from(a.1));
    let (bx, by) = (i64::from(b.0), i64::from(b.1));
    let (dx, dy) = (bx - ax, by - ay);
    let len_sq = dx * dx + dy * dy;
    // The point of the segment nearest to p, as a fraction t / len_sq.
    let t = if len_sq == 0 {
        0
    } else {
        ((px - ax) * dx + (py - ay) * dy).clamp(0, len_sq)
    };
    let (nx, ny) = if len_sq == 0 {
        (ax, ay)
    } else {
        (ax + dx * t / len_sq, ay + dy * t / len_sq)
    };
    (px - nx) * (px - nx) + (py - ny) * (py - ny)
}

/// `hit` with every dot near the finger's path from `from` (where it was at
/// the last reading, if it was down) to `to` added.
pub fn mark(hit: u32, from: Option<Point>, to: Point) -> u32 {
    let b = (i32::from(to.x), i32::from(to.y));
    let a = from.map_or(b, |f| (i32::from(f.x), i32::from(f.y)));
    let r = i64::from(HIT_RADIUS);
    DOT_AT
        .iter()
        .enumerate()
        .filter(|(_, &(x, y))| dist_sq_to_segment((i32::from(x), i32::from(y)), a, b) <= r * r)
        .fold(hit, |h, (i, _)| h | 1 << i)
}

/// True once enough dots are traced.
pub fn done(hit: u32) -> bool {
    (hit & ALL).count_ones() >= NEEDED
}

/// The first dot not traced yet, for a hint.
pub fn first_missing(hit: u32) -> Option<u8> {
    (0..DOTS)
        .find(|i| hit & 1 << i == 0)
        .and_then(|i| u8::try_from(i).ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(i: usize) -> Point {
        Point::new(DOT_AT[i].0, DOT_AT[i].1)
    }

    #[test]
    fn a_touch_traces_the_dot_under_it_and_no_other() {
        assert_eq!(mark(0, None, at(3)), 1 << 3);
        assert_eq!(mark(0, None, Point::new(20, 20)), 0, "far away");
    }

    #[test]
    fn a_quick_move_traces_every_dot_it_passes() {
        // From dot 1 to dot 3 in one reading: dot 2 in between counts too.
        // Dot 23 sits just across the dip from dot 1, close enough to count.
        assert_eq!(mark(0, Some(at(1)), at(3)), 0b1110 | 1 << 23);
    }

    #[test]
    fn going_round_the_heart_finishes_it() {
        let mut hit = 0;
        let mut last = None;
        for i in 0..=DOTS {
            let p = at(i % DOTS);
            hit = mark(hit, last, p);
            last = Some(p);
        }
        assert_eq!(hit, ALL);
        assert!(done(hit));
    }

    #[test]
    fn a_few_missed_dots_are_fine_but_not_too_many() {
        let missing_three = ALL & !(1 << 2 | 1 << 9 | 1 << 17);
        assert!(done(missing_three));
        let missing_four = missing_three & !(1 << 20);
        assert!(!done(missing_four));
    }

    #[test]
    fn the_hint_points_at_the_first_dot_left() {
        assert_eq!(first_missing(0), Some(0));
        assert_eq!(first_missing(0b111), Some(3));
        assert_eq!(first_missing(ALL), None);
    }

    #[test]
    fn the_dots_are_evenly_spaced_and_never_too_close() {
        for i in 0..DOTS {
            let (a, b) = (DOT_AT[i], DOT_AT[(i + 1) % DOTS]);
            let d = i32::from(a.0 - b.0).pow(2) + i32::from(a.1 - b.1).pow(2);
            assert!((18 * 18..=28 * 28).contains(&d), "dots {i} and next: {d}");
        }
    }
}
