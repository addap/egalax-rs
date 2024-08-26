//! Representation of screen geometry.

use serde::{Deserialize, Serialize};
use std::{
    cmp::{max, min},
    fmt,
    ops::Sub,
};

use crate::units::*;

/// A point of two coordinates in X and Y dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Point2D {
    pub x: dimX,
    pub y: dimY,
}

/// A vector of two coordinates in X and Y dimensions.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Vec2D {
    pub x: dimX,
    pub y: dimY,
}

impl Point2D {
    /// Computes the Euclidean distance between two points.
    pub fn euclidean_distance_to(&self, other: &Self) -> f32 {
        let dx = (other.x - self.x).value().abs();
        let dy = (other.y - self.y).value().abs();

        (dx.powi(2) + dy.powi(2)).sqrt()
    }

    /// Computes the Manhattan distance between two points.
    pub fn manhattan_distance_to(&self, other: &Self) -> f32 {
        let dx = (other.x - self.x).value().abs();
        let dy = (other.y - self.y).value().abs();

        dx + dy
    }

    /// A point's location vector from the origin.
    pub fn as_vec(&self) -> Vec2D {
        Vec2D {
            x: self.x,
            y: self.y,
        }
    }
}

impl Vec2D {
    /// A vector's point as a translation from the origin.
    pub fn as_point(&self) -> Point2D {
        Point2D {
            x: self.x,
            y: self.y,
        }
    }

    /// Computes the magnitude of Vector.
    pub fn magnitude(&self) -> f32 {
        (self.x.value().powi(2) + self.y.value().powi(2)).sqrt()
    }
}

impl fmt::Display for Point2D {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let description = format!("(x: {}, y: {})", self.x, self.y);
        f.write_str(&description)
    }
}

impl From<(dimX, dimY)> for Point2D {
    fn from((x, y): (dimX, dimY)) -> Self {
        Point2D { x, y }
    }
}

impl From<(UdimRepr, UdimRepr)> for Point2D {
    fn from((x, y): (UdimRepr, UdimRepr)) -> Self {
        Point2D {
            x: x.into(),
            y: y.into(),
        }
    }
}

impl From<(dimX, dimY)> for Vec2D {
    fn from((x, y): (dimX, dimY)) -> Self {
        Vec2D { x, y }
    }
}

impl From<(UdimRepr, UdimRepr)> for Vec2D {
    fn from((x, y): (UdimRepr, UdimRepr)) -> Self {
        Vec2D {
            x: x.into(),
            y: y.into(),
        }
    }
}

impl Sub for Point2D {
    type Output = Vec2D;

    fn sub(self, rhs: Self) -> Self::Output {
        Vec2D {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

/// A range of values between a minimum and maximum.
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Range<D: Dim> {
    pub min: udim<D>,
    pub max: udim<D>,
}

impl<D: Dim> Range<D> {
    /// Computes the linear factor of a value inside a range.
    pub fn linear_factor(&self, x: udim<D>) -> udim<D> {
        // solve for t
        // x = t * min + (1 - t) * max
        // => t = (max - x)/(max - min)
        if self.max == self.min {
            f32::NAN.into()
        } else {
            let t = (self.max - x) / (self.max - self.min);
            t
        }
    }

    /// Computes a linear interpolation in a range.
    pub fn lerp(&self, t: udim<D>) -> udim<D> {
        self.min * t + self.max * (udim::from(1.0) - t)
    }

    /// Computes the midpoint of a range.
    pub fn midpoint(&self) -> udim<D> {
        self.lerp(0.5.into())
    }
}

impl<D: Dim> fmt::Display for Range<D> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let description = format!("({}, {})", self.min, self.max);
        f.write_str(&description)
    }
}

impl<D: Dim> From<(udim<D>, udim<D>)> for Range<D> {
    fn from((min, max): (udim<D>, udim<D>)) -> Self {
        Range { min, max }
    }
}

impl<D: Dim> From<(UdimRepr, UdimRepr)> for Range<D> {
    fn from((min, max): (UdimRepr, UdimRepr)) -> Self {
        Range {
            min: min.into(),
            max: max.into(),
        }
    }
}

/// An axis-aligned bounding box consisting of an upper left corner (x1, y1) and lower right corner (x2, y2)
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
        AABB {
            x1: min(x1, x2),
            y1: min(y1, y2),
            x2: max(x1, x2),
            y2: max(y1, y2),
        }
    }

    /// Create a new AABB from the upper left corner and a width & height.
    pub fn new_wh(x: dimX, y: dimY, width: dimX, height: dimY) -> Self {
        AABB::new(x, y, x + width, y + height)
    }

    /// Combines two AABB's by creating the smallest AABB that contains both.
    pub fn union(self, rhs: Self) -> Self {
        AABB {
            x1: min(self.x1, rhs.x1),
            y1: min(self.y1, rhs.y1),
            x2: max(self.x2, rhs.x2),
            y2: max(self.y2, rhs.y2),
        }
    }

    /// Grows the AABB so that it also contains point.
    pub fn grow_to_point(self, point: &Point2D) -> Self {
        AABB {
            x1: min(self.x1, point.x),
            y1: min(self.y1, point.y),
            x2: max(self.x2, point.x),
            y2: max(self.y2, point.y),
        }
    }

    /// Shift x1, x2 by x and y1, y2 by y
    pub fn translate(self, x: dimX, y: dimY) -> Self {
        AABB::new(self.x1 + x, self.y1 + y, self.x2 + x, self.y2 + y)
    }

    /// Returns the AABB's range in the X dimension.
    pub fn x(&self) -> Range<X> {
        Range {
            min: self.x1.into(),
            max: self.x2.into(),
        }
    }

    /// Returns the AABB's range in the Y dimension.
    pub fn y(&self) -> Range<Y> {
        Range {
            min: self.y1.into(),
            max: self.y2.into(),
        }
    }

    /// Returns the AABB's width.
    pub fn width(&self) -> dimX {
        self.x2 - self.x1
    }

    /// Returns the AABB's height.
    pub fn height(&self) -> dimY {
        self.y2 - self.y1
    }

    /// Returns the AABB's midpoint.
    pub fn midpoint(&self) -> Point2D {
        Point2D {
            x: self.x().midpoint(),
            y: self.y().midpoint(),
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
        let description = format!(
            "ul: ({}, {})\tlr: ({}, {})",
            self.x1, self.y1, self.x2, self.y2
        );
        f.write_str(&description)
    }
}

impl From<&xrandr::Monitor> for AABB {
    fn from(m: &xrandr::Monitor) -> Self {
        AABB::new_wh(
            m.x.into(),
            m.y.into(),
            m.width_px.into(),
            m.height_px.into(),
        )
    }
}

impl From<Point2D> for AABB {
    fn from(p: Point2D) -> Self {
        AABB::new(p.x, p.y, p.x, p.y)
    }
}

impl From<(UdimRepr, UdimRepr, UdimRepr, UdimRepr)> for AABB {
    fn from((x1, y1, x2, y2): (UdimRepr, UdimRepr, UdimRepr, UdimRepr)) -> Self {
        AABB::new(x1.into(), y1.into(), x2.into(), y2.into())
    }
}
