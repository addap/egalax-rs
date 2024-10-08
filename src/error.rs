//! Our application errors.

use std::{io, time};
use thiserror::Error;

use crate::units::DimE;

/// General error type.
#[derive(Error, Debug)]
pub enum EgalaxError {
    #[error("Device Error")]
    Device,
    #[error("Monitor \"{0}\" not found")]
    MonitorNotFound(String),
    #[error("{0}")]
    Time(#[from] time::SystemTimeError),
    #[error("{0}")]
    Parse(#[from] ParsePacketError),
    #[error("{0}")]
    IO(#[from] io::Error),
    #[error("{0}")]
    Xrandr(#[from] xrandr::XrandrError),
    #[error("{0}")]
    Generic(#[from] anyhow::Error),
}

/// Errors that can happen during parsing of a packet
#[derive(Error, Debug, PartialEq, Eq)]
pub enum ParsePacketError {
    #[error("Unexpected packet tag: {0}")]
    UnexpectedTag(u8),
    #[error("{0:?} value is out of range of given resolution")]
    WrongResolution(DimE),
}
