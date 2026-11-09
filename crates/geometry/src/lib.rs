//! Points, rectangles and hit testing in integer pixels.
//!
//! Generic on purpose: nothing here knows about any one screen or project.

#![no_std]
#![warn(missing_docs)]

pub mod place;

/// A point on a screen. The origin is the top-left corner.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Point {
    pub x: i16,
    pub y: i16,
}

impl Point {
    /// Creates a point.
    pub const fn new(x: i16, y: i16) -> Self {
        Self { x, y }
    }
}

/// An axis-aligned rectangle.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct Rect {
    /// Left edge.
    pub x: i16,
    /// Top edge.
    pub y: i16,
    /// Width.
    pub w: i16,
    /// Height.
    pub h: i16,
}

impl Rect {
    /// Creates a rectangle.
    pub const fn new(x: i16, y: i16, w: i16, h: i16) -> Self {
        Self { x, y, w, h }
    }

    /// True if the point is inside or on the border.
    pub fn contains(&self, p: Point) -> bool {
        p.x >= self.x && p.x <= self.x + self.w && p.y >= self.y && p.y <= self.y + self.h
    }

    /// Squared distance from the point to the nearest part of the rectangle.
    /// Zero when the point is inside. Squared so no square root is needed.
    pub fn dist_sq(&self, p: Point) -> i32 {
        let dx = i32::from(self.x - p.x)
            .max(0)
            .max(i32::from(p.x - (self.x + self.w)));
        let dy = i32::from(self.y - p.y)
            .max(0)
            .max(i32::from(p.y - (self.y + self.h)));
        dx * dx + dy * dy
    }

    /// True if the two rectangles overlap once each is padded by `pad` pixels.
    pub fn overlaps(&self, other: &Rect, pad: i16) -> bool {
        self.x < other.x + other.w + pad
            && self.x + self.w + pad > other.x
            && self.y < other.y + other.h + pad
            && self.y + self.h + pad > other.y
    }
}
