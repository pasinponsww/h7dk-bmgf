//! A heart shape worked out row by row, for the heart wipe between scenes.
//!
//! The heart is two round lobes on top of a point. For any row, [`spans`]
//! gives the columns inside it, so [`fill_outside`] can cover the rest of
//! the row with a few rectangle fills instead of testing every pixel.
//!
//! In units of the heart's `size`: the lobes are circles of radius 0.55 at
//! (-0.5, -0.3) and (0.5, -0.3) from the center, and the sides run at 45
//! degrees down to the point at 0.95 below the center, taking over from the
//! lobes just below their middles, where the two meet smoothly.

use embedded_graphics::pixelcolor::Rgb565;
use embedded_graphics::prelude::{DrawTarget, Point, Size};
use embedded_graphics::primitives::Rectangle;

/// The columns of row `y` inside a heart of `size` centered on `center`, as
/// inclusive `(first, last)` pairs: none above and below it, two across the
/// lobes where they have not met yet, otherwise one.
pub fn spans(center: Point, size: i32, y: i32) -> [Option<(i32, i32)>; 2] {
    if size <= 0 {
        return [None, None];
    }
    let (cx, cy) = (center.x, center.y);
    let half = size / 2;
    let r = size * 11 / 20;
    let lobes_y = cy - size * 3 / 10;
    let sides_from = cy + size * 3 / 40;
    let tip = cy + size * 19 / 20;
    if y > tip {
        return [None, None];
    }
    let dy = y - lobes_y;
    // How far each lobe reaches either side of its center on this row.
    let lobe = (dy * dy <= r * r).then(|| {
        let left = u32::try_from(r * r - dy * dy).unwrap_or(0);
        i32::try_from(left.isqrt()).unwrap_or(0)
    });
    if y < lobes_y {
        // The top: two lobes, joined once they touch.
        let Some(dx) = lobe else {
            return [None, None];
        };
        let left = (cx - half - dx, cx - half + dx);
        let right = (cx + half - dx, cx + half + dx);
        return if left.1 + 1 >= right.0 {
            [Some((left.0, right.1)), None]
        } else {
            [Some(left), Some(right)]
        };
    }
    // Below the lobes' middles the heart is solid, as wide as the lobes or
    // the sloping sides, whichever reaches further.
    let lobes = lobe.map_or(0, |dx| half + dx);
    let sides = if y >= sides_from { tip - y } else { 0 };
    let w = lobes.max(sides);
    [Some((cx - w, cx + w)), None]
}

/// The size a heart centered on `center` needs to cover a `width` by
/// `height` screen completely.
pub fn covering_size(center: Point, width: i32, height: i32) -> i32 {
    let dx = center.x.max(width - center.x);
    let dy = center.y.max(height - center.y);
    (dx + dy) * 8 / 5
}

/// Fills everything inside the target's bounding box that lies outside the
/// heart with `color`.
pub fn fill_outside<D>(
    target: &mut D,
    center: Point,
    size: i32,
    color: Rgb565,
) -> Result<(), D::Error>
where
    D: DrawTarget<Color = Rgb565>,
{
    let clip = target.bounding_box();
    let x0 = clip.top_left.x;
    let x1 = x0 + i32::try_from(clip.size.width).unwrap_or(0);
    let y0 = clip.top_left.y;
    let y1 = y0 + i32::try_from(clip.size.height).unwrap_or(0);
    let run = |target: &mut D, y: i32, from: i32, to: i32| {
        let (from, to) = (from.max(x0), to.min(x1));
        if from >= to {
            return Ok(());
        }
        let width = u32::try_from(to - from).unwrap_or(0);
        target.fill_solid(
            &Rectangle::new(Point::new(from, y), Size::new(width, 1)),
            color,
        )
    };
    for y in y0..y1 {
        let mut x = x0;
        for (first, last) in spans(center, size, y).into_iter().flatten() {
            run(target, y, x, first)?;
            x = x.max(last + 1);
        }
        run(target, y, x, x1)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::vec;

    use embedded_graphics::pixelcolor::RgbColor;

    use super::*;
    use crate::frame::{Band, HEIGHT, WIDTH};
    use crate::theme::to_raw;

    fn inside(center: Point, size: i32, x: i32, y: i32) -> bool {
        spans(center, size, y)
            .into_iter()
            .flatten()
            .any(|(a, b)| (a..=b).contains(&x))
    }

    #[test]
    fn the_heart_has_a_middle_two_lobes_and_a_point() {
        let c = Point::new(100, 100);
        assert!(inside(c, 80, 100, 100), "the center");
        assert!(inside(c, 80, 60, 76), "the left lobe");
        assert!(inside(c, 80, 140, 76), "the right lobe");
        assert!(!inside(c, 80, 100, 50), "the dip between the lobes");
        assert!(inside(c, 80, 100, 170), "just above the point");
        assert!(!inside(c, 80, 100, 180), "below the point");
        assert!(!inside(c, 80, 60, 160), "outside the sloping side");
        assert!(!inside(c, 80, 100, 30), "above the lobes");
    }

    #[test]
    fn the_heart_is_the_same_on_both_sides() {
        let c = Point::new(200, 120);
        for y in 40..220 {
            for dx in 0..120 {
                assert_eq!(
                    inside(c, 100, 200 - dx, y),
                    inside(c, 100, 200 + dx, y),
                    "row {y}, {dx} from the middle"
                );
            }
        }
    }

    #[test]
    fn every_row_of_the_body_is_solid() {
        let c = Point::new(240, 136);
        for y in 136 - 30..136 + 90 {
            let s = spans(c, 100, y);
            assert!(s[0].is_some() && s[1].is_none(), "row {y}: {s:?}");
        }
    }

    #[test]
    fn a_covering_heart_covers_every_corner() {
        for center in [Point::new(240, 136), Point::new(0, 0), Point::new(470, 260)] {
            let size = covering_size(center, 480, 272);
            for (x, y) in [(0, 0), (479, 0), (0, 271), (479, 271), (240, 0), (240, 271)] {
                assert!(inside(center, size, x, y), "{center:?}: ({x}, {y})");
            }
        }
    }

    #[test]
    fn fill_outside_leaves_only_the_heart() {
        let mut buf = vec![0u16; WIDTH * HEIGHT];
        let c = Point::new(240, 136);
        fill_outside(&mut Band::new(&mut buf, 0), c, 120, Rgb565::RED).unwrap();
        for y in 0..HEIGHT {
            for x in 0..WIDTH {
                let (xi, yi) = (i32::try_from(x).unwrap(), i32::try_from(y).unwrap());
                let filled = buf[y * WIDTH + x] == to_raw(Rgb565::RED);
                assert_eq!(filled, !inside(c, 120, xi, yi), "({x}, {y})");
            }
        }
    }

    #[test]
    fn nothing_shows_through_a_heart_of_size_zero() {
        let mut buf = vec![0u16; WIDTH * 4];
        fill_outside(
            &mut Band::new(&mut buf, 100),
            Point::new(240, 136),
            0,
            Rgb565::RED,
        )
        .unwrap();
        assert!(buf.iter().all(|&p| p == to_raw(Rgb565::RED)));
    }
}
