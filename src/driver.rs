use crate::protocol::{Packet, ParsePacketError, RawPacket, RAW_PACKET_LEN};
use crate::{dimX, dimY, Point};
use std::{error, fmt, io};

#[derive(Debug, PartialEq)]
struct Driver {
    touch_state: TouchState,
    ul_bounds: Point,
    lr_bounds: Point,
    monitor_info: MonitorInfo,
}

impl Driver {
    fn new(ul_bounds: Point, lr_bounds: Point) -> Self {
        Self {
            touch_state: TouchState::default(),
            ul_bounds,
            lr_bounds,
            monitor_info: MonitorInfo::default(),
        }
    }

    // TODO implement debouncing
    fn update(&mut self, packet: Packet) -> Vec<ChangeSet> {
        let mut changes = Vec::new();

        if self.touch_state.is_touching != packet.is_touching() {
            self.touch_state.is_touching = packet.is_touching();
            if packet.is_touching() {
                changes.push(ChangeSet::Pressed);
            } else {
                changes.push(ChangeSet::Released);
            }
        }

        if self.touch_state.x() != packet.x() {
            self.touch_state.set_x(packet.x());
            changes.push(ChangeSet::ChangedX(packet.x()));
        }

        if self.touch_state.y() != packet.y() {
            self.touch_state.set_y(packet.y());
            changes.push(ChangeSet::ChangedY(packet.y()));
        }

        changes
    }
}

/// Changes for which we need to generate evdev events after we processed a packet
// TODO does it make sense to collapse ChangedX & ChangedY into a Changed(T, udim<T>)? Probably not possible
#[derive(Debug, PartialEq)]
enum ChangeSet {
    ChangedX(dimX),
    ChangedY(dimY),
    Pressed,
    Released,
}

impl ChangeSet {
    fn send_event(changes: &[ChangeSet]) -> Result<(), EgalaxError> {
        println!("Sending event {:#?}", changes);
        Ok(())
    }
}

#[derive(Debug, PartialEq)]
struct TouchState {
    is_touching: bool,
    p: Point,
}

impl TouchState {
    pub fn is_touching(&self) -> bool {
        self.is_touching
    }

    pub fn x(&self) -> dimX {
        self.p.x
    }

    pub fn set_x(&mut self, x: dimX) -> () {
        self.p.x = x;
    }

    pub fn y(&self) -> dimY {
        self.p.y
    }

    pub fn set_y(&mut self, y: dimY) -> () {
        self.p.y = y;
    }
}

impl Default for TouchState {
    fn default() -> Self {
        TouchState {
            is_touching: false,
            p: (0, 0).into(),
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
            ul: (0, 0).into(),
            lr: (1000, 1000).into(),
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
pub fn process_packets<T, F>(stream: &mut T, f: &mut F) -> Result<(), EgalaxError>
where
    T: io::Read,
    F: FnMut(Packet) -> Result<(), EgalaxError>,
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
        f(packet)?;
    }
}

/// Print the sequence of packets in the given stream
pub fn print_packets(stream: &mut impl io::Read) -> Result<(), EgalaxError> {
    process_packets(stream, &mut |packet| Ok(println!("{:#?}", packet)))
}

pub fn virtual_mouse(stream: &mut impl io::Read) -> Result<(), EgalaxError> {
    let ul_bounds = (300, 300).into();
    let lr_bounds = (3800, 3800).into();
    let mut state = Driver::new(ul_bounds, lr_bounds);

    let mut process_packet = |packet| {
        let changes = state.update(packet);
        ChangeSet::send_event(&changes)
    };
    process_packets(stream, &mut process_packet)
}
