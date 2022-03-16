pub mod driver;
pub mod protocol;

use std::{fmt, marker::PhantomData};

pub trait Dim {}
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct DimX;
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct DimY;
struct DimAny;

impl Dim for DimX {}
impl Dim for DimY {}
impl Dim for DimAny {}

/// Integer type of a screen dimension
type UdimRepr = u16;
/// Public wrapper which uses PhantomData over Dim to statically tell apart x and y of monitor.
#[allow(non_camel_case_types)]
#[repr(transparent)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct udim<T: Dim>(PhantomData<T>, UdimRepr);
#[allow(non_camel_case_types)]
pub type dimX = udim<DimX>;
#[allow(non_camel_case_types)]
pub type dimY = udim<DimY>;

#[derive(Debug, PartialEq)]
pub struct Point {
    pub x: dimX,
    pub y: dimY,
}

impl<T: Dim> udim<T> {
    pub fn value(&self) -> i32 {
        self.1 as i32
    }

    pub fn linear_factor(&self, a: udim<T>, b: udim<T>) -> f64 {
        // solve for t
        // self = t * a + (1 - t) * b
        // => t = (b - self)/(b - a)
        // println!("a: {}\tb: {}\tc: {}", a, b, self);
        let t = ((b.value() - self.value()) as f64) / ((b.value() - a.value()) as f64);
        // println!("linear factor: {}", t);
        t
    }

    pub fn lerp(a: udim<T>, b: udim<T>, t: f64) -> udim<T> {
        // TODO clamped arithmetic so that we cannot end up with negative values
        ((a.value() as f64) * t + (b.value() as f64) * (1.0 - t)).into()
    }
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

impl<T: Dim> fmt::Display for udim<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.1.fmt(f)
    }
}

impl fmt::Display for Point {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let description = format!("(x: {}, y: {})", self.x, self.y);
        f.write_str(&description)
    }
}

impl<T: Dim> From<u16> for udim<T> {
    fn from(c: u16) -> Self {
        udim(PhantomData, c as UdimRepr)
    }
}

/// Just for the tests where we use string literals
impl<T: Dim> From<i32> for udim<T> {
    fn from(c: i32) -> Self {
        udim(PhantomData, c as UdimRepr)
    }
}

/// Used in Lerp functions
/// TODO maybe round
impl<T: Dim> From<f64> for udim<T> {
    fn from(c: f64) -> Self {
        udim(PhantomData, c as UdimRepr)
    }
}

impl<T: Dim> From<&udim<T>> for i32 {
    fn from(d: &udim<T>) -> Self {
        d.1 as i32
    }
}

impl From<(dimX, dimY)> for Point {
    fn from((x, y): (dimX, dimY)) -> Self {
        Point { x, y }
    }
}

impl From<(i32, i32)> for Point {
    fn from((x, y): (i32, i32)) -> Self {
        Point {
            x: x.into(),
            y: y.into(),
        }
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
