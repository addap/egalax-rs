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
    ops::{Add, Sub},
};

use crate::geo::Range;

/// X dimension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct X;

/// Y dimension.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Y;

/// Marker trait that represents a dimension.
/// Effectively, this declares a new kind with two type constructors.
pub trait Dim: Eq + Ord {}
impl Dim for X {}
impl Dim for Y {}

/// Integer type of a screen dimension
pub type UdimRepr = i32;

/// Wrapper which uses PhantomData to statically tell apart numbers of different dimensions.
#[allow(non_camel_case_types)]
#[repr(transparent)]
#[derive(Debug, PartialEq, Eq, PartialOrd, Clone, Copy)]
pub struct udim<T: Dim>(PhantomData<T>, UdimRepr);

/// Number in X dimension.
#[allow(non_camel_case_types)]
pub type dimX = udim<X>;

/// Number in X dimension.
#[allow(non_camel_case_types)]
pub type dimY = udim<Y>;

impl<T: Dim> udim<T> {
    /// The underlying dimensionless value.
    pub fn value(&self) -> UdimRepr {
        self.1
    }

    /// Takes the arithmetic average of two dimensioned numbers.
    pub fn average(x: Self, y: Self) -> Self {
        Range::from((x, y)).midpoint()
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

impl<T: Dim> From<i32> for udim<T> {
    fn from(c: i32) -> Self {
        udim(PhantomData, c as UdimRepr)
    }
}

impl<T: Dim> From<udim<T>> for UdimRepr {
    fn from(d: udim<T>) -> Self {
        d.1
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

impl<T: Dim + PartialOrd + Eq> Ord for udim<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.1.cmp(&other.1)
    }
}

impl<T: Dim> Serialize for udim<T> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        self.1.serialize(serializer)
    }
}

impl<'de, T: Dim> Deserialize<'de> for udim<T> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
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
