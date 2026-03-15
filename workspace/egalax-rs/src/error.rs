//! Our application errors.

use thiserror::Error;

use crate::units::DimE;

/// Errors that can happen during parsing of a packet
#[derive(Error, Debug, PartialEq, Eq)]
pub enum ParsePacketError {
    #[error("Unexpected packet tag: {0}")]
    UnexpectedTag(u8),
    #[error("{0:?} value is out of range of given resolution")]
    WrongResolution(DimE),
}

#[derive(Error, Debug, PartialEq, Eq)]
#[error("Failed to serialize config file.")]
pub struct ConfigSerializeError(#[from] toml::ser::Error);

#[derive(Error, Debug, PartialEq, Eq)]
#[error("Failed to parse config file.")]
pub struct ConfigParseError(#[from] toml::de::Error);
