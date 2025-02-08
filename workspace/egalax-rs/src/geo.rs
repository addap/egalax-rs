//! Representation of screen geometry.
#![allow(clippy::cast_precision_loss, clippy::wildcard_imports)]

use serde::{Deserialize, Serialize};
use std::{
    cmp::{max, min},
    fmt,
};

use crate::units::*;

/// A point of two coordinates in X and Y dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point2D {
    pub x: dimX,
    pub y: dimY,
}

impl Point2D {
    /// Computes the Euclidean distance between two points.
    pub fn euclidean_distance_to(&self, other: &Self) -> f32 {
        let dx = (other.x - self.x).value();
        let dy = (other.y - self.y).value();

        ((dx * dx + dy * dy) as f32).sqrt()
    }

    /// Computes the Manhattan distance between two points.
    pub fn manhattan_distance_to(&self, other: &Self) -> f32 {
        let dx = (other.x - self.x).value().abs();
        let dy = (other.y - self.y).value().abs();

        (dx + dy) as f32
    }

    /// Computes the magnitude of the origin vector.
    pub fn vec_magnitude(&self) -> f32 {
        self.euclidean_distance_to(&(0, 0).into())
    }
}

impl fmt::Display for Point2D {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "(x: {}, y: {})", self.x, self.y)
    }
}

/// Generic From instance to convert various things into [`Point2D`].
impl<T: Into<dimX> + Into<dimY>> From<(T, T)> for Point2D {
    fn from((x, y): (T, T)) -> Self {
        Self {
            x: x.into(),
            y: y.into(),
        }
    }
}

/// A range of values between a minimum and maximum.
/// The fields are private to uphold the invariant that min <= max.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Range<D: Dim> {
    min: udim<D>,
    max: udim<D>,
}

impl<D: Dim> Range<D> {
    /// Creates a new Range between x1 and x2.
    pub fn new(x1: udim<D>, x2: udim<D>) -> Self {
        Self {
            min: min(x1, x2),
            max: max(x1, x2),
        }
    }

    /// Returns the minimum value of the Range.
    pub fn min(&self) -> udim<D> {
        self.min
    }

    /// Returns the maximum value of the Range.
    pub fn max(&self) -> udim<D> {
        self.max
    }

    pub fn clamp(&self, v: udim<D>) -> udim<D> {
        min(self.max, max(self.min, v))
    }

    /// Returns the length of a Range.
    pub fn length(&self) -> udim<D> {
        self.max - self.min
    }

    /// Computes the linear factor of a value inside a range.
    pub fn linear_factor(&self, x: udim<D>) -> f32 {
        // x = (1 - t) * min + t * max
        // solve for t
        // => t = (x - min)/(max - min)
        if self.max == self.min {
            0.0
        } else {
            (x - self.min).float() / (self.max - self.min).float()
        }
    }

    /// Computes a linear interpolation in a range.
    pub fn lerp(&self, t: f32) -> udim<D> {
        self.min * (1.0 - t) + self.max * t
    }

    /// Computes the midpoint of a range.
    pub fn midpoint(&self) -> udim<D> {
        self.lerp(0.5)
    }
}

impl<D: Dim> fmt::Display for Range<D> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "({}, {})", self.min, self.max)
    }
}

/// Generic From instance to convert various things into Ranges.
impl<D: Dim, T: Into<udim<D>>> From<(T, T)> for Range<D> {
    fn from((min, max): (T, T)) -> Self {
        Range {
            min: min.into(),
            max: max.into(),
        }
    }
}

/// An axis-aligned bounding box consisting of an upper-left corner (x1, y1) and lower-right corner (x2, y2)
/// This assumes that x coordinates grow to the right and y coordinates grow downward.
#[derive(Debug, PartialEq, Clone, Copy, Serialize, Deserialize)]
pub struct AABB {
    x1: dimX,
    y1: dimY,
    x2: dimX,
    y2: dimY,
}

impl AABB {
    /// Create a new AABB given the coordinates of the endpoints.
    pub fn new(x1: dimX, y1: dimY, x2: dimX, y2: dimY) -> Self {
        Self {
            x1: min(x1, x2),
            y1: min(y1, y2),
            x2: max(x1, x2),
            y2: max(y1, y2),
        }
    }

    /// Create a new AABB from the upper-left corner and a width & height.
    pub fn new_wh(x: dimX, y: dimY, width: dimX, height: dimY) -> Self {
        Self::new(x, y, x + width, y + height)
    }

    /// Combines two AABBs by creating the smallest AABB that contains both.
    #[must_use]
    pub fn union(self, rhs: Self) -> Self {
        Self {
            x1: min(self.x1, rhs.x1),
            y1: min(self.y1, rhs.y1),
            x2: max(self.x2, rhs.x2),
            y2: max(self.y2, rhs.y2),
        }
    }

    /// Grows the AABB so that it also contains point.
    #[must_use]
    pub fn grow_to_point(self, point: &Point2D) -> Self {
        Self {
            x1: min(self.x1, point.x),
            y1: min(self.y1, point.y),
            x2: max(self.x2, point.x),
            y2: max(self.y2, point.y),
        }
    }

    /// Shift x1, x2 by x and y1, y2 by y
    #[must_use]
    pub fn translate(self, x: dimX, y: dimY) -> Self {
        Self::new(self.x1 + x, self.y1 + y, self.x2 + x, self.y2 + y)
    }

    /// Returns the AABB's range in the X dimension.
    pub fn xrange(&self) -> Range<X> {
        Range::new(self.x1, self.x2)
    }

    /// Returns the AABB's range in the Y dimension.
    pub fn yrange(&self) -> Range<Y> {
        Range::new(self.y1, self.y2)
    }

    pub fn clamp(&self, p: Point2D) -> Point2D {
        Point2D {
            x: self.xrange().clamp(p.x),
            y: self.yrange().clamp(p.y),
        }
    }

    /// Returns the AABB's width.
    pub fn width(&self) -> dimX {
        self.xrange().length()
    }

    /// Returns the AABB's height.
    pub fn height(&self) -> dimY {
        self.yrange().length()
    }

    /// Returns the AABB's midpoint.
    pub fn midpoint(&self) -> Point2D {
        Point2D {
            x: self.xrange().midpoint(),
            y: self.yrange().midpoint(),
        }
    }
}

impl Default for AABB {
    fn default() -> Self {
        Self {
            x1: 0.into(),
            y1: 0.into(),
            x2: 0.into(),
            y2: 0.into(),
        }
    }
}

impl fmt::Display for AABB {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "upper-left: ({}, {})\tlower-right: ({}, {})",
            self.x1, self.y1, self.x2, self.y2
        )
    }
}

/// Generic From instance to convert various things into AABBs.
impl<T: Into<dimX> + Into<dimY>> From<(T, T, T, T)> for AABB {
    fn from((x1, y1, x2, y2): (T, T, T, T)) -> Self {
        Self::new(x1.into(), y1.into(), x2.into(), y2.into())
    }
}
