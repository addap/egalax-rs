use serde::{Deserialize, Serialize};
use std::{
    cmp::{max, min},
    fmt,
    ops::{Add, Sub},
};

use crate::units::*;

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Point {
    pub x: dimX,
    pub y: dimY,
}

impl Point {
    pub fn euc_distance_to(&self, other: &Self) -> f64 {
        let dx = ((other.x.value() as f64) - (self.x.value() as f64)).abs();
        let dy = ((other.y.value() as f64) - (self.y.value() as f64)).abs();

        (dx.powi(2) + dy.powi(2)).sqrt()
    }

    pub fn manhat_distance_to(&self, other: &Self) -> i32 {
        let dx = (other.x.value() - self.x.value()).abs();
        let dy = (other.y.value() - self.y.value()).abs();

        dx + dy
    }

    pub fn magnitude(&self) -> f64 {
        self.euc_distance_to(&(0, 0).into())
    }
}

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let description = format!("(x: {}, y: {})", self.x, self.y);
        f.write_str(&description)
    }
}

impl From<(dimX, dimY)> for Point {
    fn from((x, y): (dimX, dimY)) -> Self {
        Point { x, y }
    }
}

impl From<(UdimRepr, UdimRepr)> for Point {
    fn from((x, y): (UdimRepr, UdimRepr)) -> Self {
        Point {
            x: x.into(),
            y: y.into(),
        }
    }
}

impl Sub for Point {
    type Output = Point;

    fn sub(self, rhs: Self) -> Self::Output {
        Point {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
        }
    }
}

#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Range<T: Dim> {
    pub min: udim<T>,
    pub max: udim<T>,
}

impl<T: Dim> Range<T> {
    pub fn linear_factor(&self, x: udim<T>) -> f64 {
        // solve for t
        // self = t * a + (1 - t) * b
        // => t = (b - self)/(b - a)
        let t = ((self.max.value() - x.value()) as f64)
            / ((self.max.value() - self.min.value()) as f64);
        t
    }

    pub fn lerp(&self, t: f64) -> udim<T> {
        ((self.min.value() as f64) * t + (self.max.value() as f64) * (1.0 - t)).into()
    }

    pub fn midpoint(&self) -> udim<T> {
        self.lerp(0.5)
    }
}

impl<T: Dim> fmt::Display for Range<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let description = format!("({}, {})", self.min, self.max);
        f.write_str(&description)
    }
}

impl<T: Dim> From<(udim<T>, udim<T>)> for Range<T> {
    fn from((min, max): (udim<T>, udim<T>)) -> Self {
        Range { min, max }
    }
}

impl<T: Dim> From<(UdimRepr, UdimRepr)> for Range<T> {
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
    pub fn new(x1: dimX, y1: dimY, x2: dimX, y2: dimY) -> Self {
        AABB {
            x1: min(x1, x2),
            y1: min(y1, y2),
            x2: max(x1, x2),
            y2: max(y1, y2),
        }
    }

    pub fn new_wh(x: dimX, y: dimY, width: dimX, height: dimY) -> Self {
        AABB::new(x, y, x + width, y + height)
    }

    pub fn union(self, rhs: Self) -> Self {
        AABB {
            x1: min(self.x1, rhs.x1),
            y1: min(self.y1, rhs.y1),
            x2: max(self.x2, rhs.x2),
            y2: max(self.y2, rhs.y2),
        }
    }

    /// Grow the AABB so that it also contains point.
    pub fn grow_to_point(self, point: &Point) -> Self {
        AABB {
            x1: min(self.x1, point.x),
            y1: min(self.y1, point.y),
            x2: max(self.x2, point.x),
            y2: max(self.y2, point.y),
        }
    }

    /// Shift x1,x2 by x and y1,y2 by y
    pub fn shift(self, x: dimX, y: dimY) -> Self {
        AABB::new(self.x1 + x, self.y1 + y, self.x2 + x, self.y2 + y)
    }

    pub fn x(&self) -> Range<DimX> {
        Range {
            min: self.x1.into(),
            max: self.x2.into(),
        }
    }

    pub fn y(&self) -> Range<DimY> {
        Range {
            min: self.y1.into(),
            max: self.y2.into(),
        }
    }

    pub fn width(&self) -> dimX {
        self.x2 - self.x1
    }

    pub fn height(&self) -> dimY {
        self.y2 - self.y1
    }

    pub fn midpoint(&self) -> Point {
        Point {
            x: self.x().midpoint(),
            y: self.y().midpoint(),
        }
    }
}

impl Add for AABB {
    type Output = AABB;

    fn add(self, rhs: Self) -> Self::Output {
        AABB {
            x1: self.x1 + rhs.x1,
            y1: self.y1 + rhs.y1,
            x2: self.x2 + rhs.x2,
            y2: self.y2 + rhs.y2,
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
        AABB::new(
            m.x.into(),
            m.y.into(),
            (m.x + m.width_px).into(),
            (m.y + m.height_px).into(),
        )
    }
}

impl From<Point> for AABB {
    fn from(p: Point) -> Self {
        AABB::new(p.x, p.y, p.x, p.y)
    }
}

impl From<(UdimRepr, UdimRepr, UdimRepr, UdimRepr)> for AABB {
    fn from((x1, y1, x2, y2): (UdimRepr, UdimRepr, UdimRepr, UdimRepr)) -> Self {
        AABB::new(x1.into(), y1.into(), x2.into(), y2.into())
    }
}
