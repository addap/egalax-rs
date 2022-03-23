use serde::{Deserialize, Serialize};
use std::{
    cmp::{max, min},
    fmt,
    ops::Add,
};

use crate::units::*;

#[derive(Debug, PartialEq)]
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
        // println!("a: {}\tb: {}\tc: {}", a, b, self);
        let t = ((self.max.value() - x.value()) as f64)
            / ((self.max.value() - self.min.value()) as f64);
        // println!("linear factor: {}", t);
        t
    }

    pub fn lerp(&self, t: f64) -> udim<T> {
        ((self.min.value() as f64) * t + (self.max.value() as f64) * (1.0 - t)).into()
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
    x1: UdimRepr,
    y1: UdimRepr,
    x2: UdimRepr,
    y2: UdimRepr,
}

impl AABB {
    pub fn new(x1: UdimRepr, y1: UdimRepr, x2: UdimRepr, y2: UdimRepr) -> Self {
        AABB { x1, y1, x2, y2 }
    }

    // TODO could use AsRef to be able to call it with a reference
    pub fn union(self, rhs: Self) -> Self {
        AABB {
            x1: min(self.x1, rhs.x1),
            y1: min(self.y1, rhs.y1),
            x2: max(self.x2, rhs.x2),
            y2: max(self.y2, rhs.y2),
        }
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
            x1: 0,
            y1: 0,
            x2: 0,
            y2: 0,
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
        AABB::new(m.x, m.y, m.x + m.width_px, m.y + m.height_px)
    }
}

// TODO implement deserialize
// impl<T: Dim> Serialize for Range<T> {
//     fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
//     where
//         S: Serializer,
//     {
//         let mut tup = serializer.serialize_tuple(2)?;
//         tup.serialize_element(&self.min.value())?;
//         tup.serialize_element(&self.max.value())?;
//         tup.end()
//     }
// }
