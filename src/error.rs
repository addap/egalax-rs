use std::time;
use std::{error, fmt, io};

#[derive(Debug)]
pub enum EgalaxError {
    UnexpectedEOF,
    DeviceError,
    MonitorNotFound(String),
    TimeError(time::SystemTimeError),
    ParseError(ParsePacketError),
    IOError(io::Error),
    Xrandr(xrandr::XrandrError),
    SerdeLexpr(serde_lexpr::Error),
}

impl From<time::SystemTimeError> for EgalaxError {
    fn from(e: time::SystemTimeError) -> Self {
        Self::TimeError(e)
    }
}

impl From<io::Error> for EgalaxError {
    fn from(e: io::Error) -> Self {
        Self::IOError(e)
    }
}

impl From<ParsePacketError> for EgalaxError {
    fn from(e: ParsePacketError) -> Self {
        Self::ParseError(e)
    }
}

impl From<xrandr::XrandrError> for EgalaxError {
    fn from(e: xrandr::XrandrError) -> Self {
        Self::Xrandr(e)
    }
}

impl From<serde_lexpr::Error> for EgalaxError {
    fn from(e: serde_lexpr::Error) -> Self {
        Self::SerdeLexpr(e)
    }
}

impl error::Error for EgalaxError {}

impl fmt::Display for EgalaxError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let description = match self {
            EgalaxError::ParseError(e) => return e.fmt(f),
            EgalaxError::IOError(e) => return e.fmt(f),
            EgalaxError::TimeError(e) => return e.fmt(f),
            EgalaxError::Xrandr(e) => return e.fmt(f),
            EgalaxError::SerdeLexpr(e) => return e.fmt(f),
            EgalaxError::UnexpectedEOF => String::from("Unexpected EOF"),
            EgalaxError::DeviceError => String::from("Device Error"),
            EgalaxError::MonitorNotFound(name) => format!("Monitor \"{}\" not found", name),
        };
        f.write_str(&description)
    }
}

/// Errors that can happen during parsing of a packet
#[derive(PartialEq, Debug)]
pub enum ParsePacketError {
    UnexpectedTag(u8),
    ResolutionErrorX,
    ResolutionErrorY,
}

impl error::Error for ParsePacketError {}

impl fmt::Display for ParsePacketError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let description = match self {
            ParsePacketError::UnexpectedTag(h) => format!("Unexpected packet tag: {}", h),
            ParsePacketError::ResolutionErrorX => {
                String::from("X value is out of range of given resolution")
            }
            ParsePacketError::ResolutionErrorY => {
                String::from("Y value is out of range of given resolution")
            }
        };
        f.write_str(&description)
    }
}
