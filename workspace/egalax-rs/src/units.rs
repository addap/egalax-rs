//! Wrapper types for number of different dimensions (x & y).
//!
//! To prevent accidentally mixing different dimensions when calculating
//! with screen geometry we add some wrapper types that restrict the
//! allowed operations.
//!
//! TODO: To go a step further we could also add types to represent normalized
//! screen-space vs pixels.

use serde::{Deserialize, Serialize};
use std::{
    fmt,
    marker::PhantomData,
    ops::{Add, Mul, Sub},
};

/// X dimension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct X;

/// Y dimension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Y;

/// Marker trait that represents a dimension.
/// Effectively, this declares a new kind with two type constructors.
pub trait Dim: Clone + Copy + Eq + Ord {}
impl Dim for X {}
impl Dim for Y {}

/// Integer type of a screen dimension
pub type UdimRepr = i32;

/// Wrapper which uses PhantomData to statically tell apart numbers of different dimensions.
#[allow(non_camel_case_types)]
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct udim<D: Dim>(PhantomData<D>, UdimRepr);

/// Number in X dimension.
#[allow(non_camel_case_types)]
pub type dimX = udim<X>;

/// Number in X dimension.
#[allow(non_camel_case_types)]
pub type dimY = udim<Y>;

impl<D: Dim> udim<D> {
    /// The underlying dimensionless value.
    pub fn value(self) -> UdimRepr {
        self.1
    }

    /// The underlying dimensionless value as an f32.
    pub fn float(self) -> f32 {
        self.value() as f32
    }
}

impl<D: Dim> fmt::Display for udim<D> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        self.1.fmt(f)
    }
}

/// Generic From instance to convert scalar values into udim<D>.
/// We use this mainly for UdimRepr and smaller types such as i16.
impl<D: Dim, T: Into<UdimRepr>> From<T> for udim<D> {
    fn from(x: T) -> Self {
        udim(PhantomData, x.into())
    }
}

/// Arithmetic instances.
impl<D: Dim> Add for udim<D> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        (self.1 + rhs.1).into()
    }
}

impl<D: Dim> Sub for udim<D> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        (self.1 - rhs.1).into()
    }
}

impl<D: Dim> Mul<f32> for udim<D> {
    type Output = udim<D>;

    fn mul(self, rhs: f32) -> Self::Output {
        ((self.1 as f32 * rhs) as UdimRepr).into()
    }
}

/// Serialization instances.
impl<D: Dim> Serialize for udim<D> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.1.serialize(serializer)
    }
}

impl<'de, D: Dim> Deserialize<'de> for udim<D> {
    fn deserialize<De>(deserializer: De) -> Result<Self, De::Error>
    where
        De: serde::Deserializer<'de>,
    {
        let x: UdimRepr = UdimRepr::deserialize(deserializer)?;
        Ok(x.into())
    }
}

/// A separate dimension enum to avoid generics in some cases.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DimE {
    X,
    Y,
}

impl From<dimX> for DimE {
    fn from(_: dimX) -> Self {
        Self::X
    }
}

impl From<dimY> for DimE {
    fn from(_: dimY) -> Self {
        Self::Y
    }
}
