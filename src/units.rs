use std::{
    fmt,
    marker::PhantomData,
    ops::{Add, Sub},
};

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
pub type UdimRepr = i32;
/// Public wrapper which uses PhantomData over Dim to statically tell apart x and y of monitor.
#[allow(non_camel_case_types)]
#[repr(transparent)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct udim<T: Dim>(PhantomData<T>, UdimRepr);
#[allow(non_camel_case_types)]
pub type dimX = udim<DimX>;
#[allow(non_camel_case_types)]
pub type dimY = udim<DimY>;

impl<T: Dim> udim<T> {
    pub fn value(&self) -> UdimRepr {
        self.1
    }
}

impl<T: Dim> fmt::Display for udim<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.1.fmt(f)
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

impl<T: Dim> From<udim<T>> for i32 {
    fn from(d: udim<T>) -> Self {
        d.1 as i32
    }
}

impl<T: Dim> Add for udim<T> {
    type Output = udim<T>;

    fn add(self, rhs: Self) -> Self::Output {
        (self.1 + rhs.1).into()
    }
}

impl<T: Dim> Sub for udim<T> {
    type Output = udim<T>;

    fn sub(self, rhs: Self) -> Self::Output {
        (self.1 - rhs.1).into()
    }
}
