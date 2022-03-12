use crate::protocol::{Packet, ParsePacketError, RawPacket, RAW_PACKET_LEN};
use std::{error, fmt, io};

type Point = (u16, u16);

#[derive(Debug, PartialEq)]
pub struct Driver {
    touch_state: TouchState,
    x_bounds: Point,
    y_bounds: Point,
    monitor_info: MonitorInfo,
}

impl Driver {
    pub fn new(x_bounds: Point, y_bounds: Point) -> Self {
        Self {
            touch_state: TouchState::default(),
            x_bounds,
            y_bounds,
            monitor_info: MonitorInfo::default(),
        }
    }
}

#[derive(Debug, PartialEq)]
struct TouchState {
    is_touching: bool,
    x: u16,
    y: u16,
}

impl Default for TouchState {
    fn default() -> Self {
        TouchState {
            is_touching: false,
            x: 0,
            y: 0,
        }
    }
}

#[derive(Debug, PartialEq)]
struct MonitorInfo {
    ul: Point,
    lr: Point,
}

// TODO need to get monitor dimensions from xrandr or config file
impl Default for MonitorInfo {
    fn default() -> Self {
        MonitorInfo {
            ul: (0, 0),
            lr: (1000, 1000),
        }
    }
}

#[derive(Debug)]
pub enum EgalaxError {
    UnexpectedEOF,
    ParseError(ParsePacketError),
    IOError(io::Error),
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

impl error::Error for EgalaxError {}

impl fmt::Display for EgalaxError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let description = match self {
            EgalaxError::ParseError(e) => return e.fmt(f),
            EgalaxError::IOError(e) => return e.fmt(f),
            EgalaxError::UnexpectedEOF => "Unexpected EOF",
        };
        f.write_str(&description)
    }
}

/// Call a function on all packets in the given stream
pub fn process_packets<T, F>(stream: &mut T, f: F) -> Result<(), EgalaxError>
where
    T: io::Read,
    F: Fn(Packet) -> (),
{
    let mut raw_packet: RawPacket = [0; RAW_PACKET_LEN];

    loop {
        let read_bytes = stream.read(&mut raw_packet)?;
        if read_bytes == 0 {
            return Ok(());
        } else if read_bytes < RAW_PACKET_LEN {
            return Err(EgalaxError::UnexpectedEOF);
        }
        let packet = Packet::try_from(raw_packet)?;
        f(packet);
    }
}

/// Print the sequence of packets in the given stream
pub fn print_packets(stream: &mut impl io::Read) -> Result<(), EgalaxError> {
    process_packets(stream, |packet| println!("{:#?}", packet))
}
