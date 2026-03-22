//! Our application errors.

use thiserror::Error;

use crate::units::DimE;

/// Errors that can happen during parsing of a HID report.
#[derive(Error, Debug, PartialEq, Eq)]
pub enum ParseReportError {
    #[error("Unexpected report number: {0}")]
    UnexpectedReportNum(u8),
    #[error("{0:?} value is out of range of given resolution")]
    WrongResolution(DimE),
}

#[derive(Error, Debug, PartialEq, Eq)]
#[error("Failed to serialize config file.")]
pub struct ConfigSerializeError(#[from] toml::ser::Error);

#[derive(Error, Debug, PartialEq, Eq)]
#[error("Failed to parse config file.")]
pub struct ConfigParseError(#[from] toml::de::Error);
