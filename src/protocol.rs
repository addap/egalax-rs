//! Implements parsing of the raw binary packets that our touchscreen sends.

use std::{error, fmt};

use crate::{dimX, dimY, DimX, DimY, Point, UdimRepr};

/// A representation of a packet sent over USB
#[derive(PartialEq, Debug)]
pub struct Packet {
    is_touching: bool,
    p: Point,
    res: u8,
}

impl Packet {
    pub fn is_touching(&self) -> bool {
        self.is_touching
    }
    pub fn x(&self) -> dimX {
        self.p.x
    }
    pub fn y(&self) -> dimY {
        self.p.y
    }
    pub fn res(&self) -> u8 {
        self.res
    }
}

pub const RAW_PACKET_LEN: usize = 6;
/// Type of raw bytes sent over USB
pub type RawPacket = [u8; RAW_PACKET_LEN];

/// Errors that can happen during parsing of a packet
#[derive(PartialEq, Debug)]
pub enum ParsePacketError {
    MalformedHeader(u8),
    ResolutionErrorX,
    ResolutionErrorY,
}

/// Constants for bit positions in the packet
mod bits {
    pub const TOUCH_BIT: u8 = 0x01;
    pub const RES_BITS: u8 = 0x06;
}

/// Parsing logic
impl TryFrom<RawPacket> for Packet {
    type Error = ParsePacketError;

    fn try_from(packet: RawPacket) -> Result<Self, Self::Error> {
        if packet[0] != 0x02 {
            return Err(ParsePacketError::MalformedHeader(packet[0]));
        }

        let res = match packet[1] & bits::RES_BITS {
            0x00 => 11,
            0x02 => 12,
            0x04 => 13,
            0x05 => 14,
            _ => unreachable!("only two bits should be left, match can never succeed"),
        };

        let is_touching = (packet[1] & bits::TOUCH_BIT) == 0x01;

        let y: UdimRepr = ((packet[3] as UdimRepr) << 8) | (packet[2] as UdimRepr);
        let x: UdimRepr = ((packet[5] as UdimRepr) << 8) | (packet[4] as UdimRepr);

        if y >> res != 0x00 {
            return Err(ParsePacketError::ResolutionErrorY);
        } else if x >> res != 0x00 {
            return Err(ParsePacketError::ResolutionErrorX);
        }

        Ok(Packet {
            is_touching,
            p: (x, y).into(),
            res,
        })
    }
}

impl error::Error for ParsePacketError {}

impl fmt::Display for ParsePacketError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let description = match self {
            ParsePacketError::MalformedHeader(h) => format!("Packet header is malformed: {}", h),
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

#[cfg(test)]
mod tests {

    use super::*;

    #[test]
    fn test_parse_touch_upper_left() {
        let raw_packet: RawPacket = [0x02, 0x03, 0x3b, 0x01, 0x32, 0x01];

        assert_eq!(
            Ok(Packet {
                is_touching: true,
                p: (306, 315).into(),
                res: 12
            }),
            Packet::try_from(raw_packet)
        );
    }

    #[test]
    fn test_parse_release_upper_left() {
        let raw_packet: RawPacket = [0x02, 0x02, 0x35, 0x01, 0x39, 0x01];

        assert_eq!(
            Ok(Packet {
                is_touching: false,
                p: (313, 309).into(),
                res: 12
            }),
            Packet::try_from(raw_packet)
        );
    }

    #[test]
    fn test_malformed_const() {
        let raw_packet: RawPacket = [0xaa, 0x02, 0x35, 0x01, 0x39, 0x01];

        assert_eq!(
            Err(ParsePacketError::MalformedHeader(0xaa)),
            Packet::try_from(raw_packet)
        );
    }

    #[test]
    fn test_malformed_res_y() {
        let raw_packet: RawPacket = [0x02, 0x02, 0x35, 0x11, 0x39, 0x01];

        assert_eq!(
            Err(ParsePacketError::ResolutionErrorY),
            Packet::try_from(raw_packet)
        );
    }

    #[test]
    fn test_malformed_res_x() {
        let raw_packet: RawPacket = [0x02, 0x02, 0x35, 0x01, 0x39, 0x11];

        assert_eq!(
            Err(ParsePacketError::ResolutionErrorX),
            Packet::try_from(raw_packet)
        );
    }
}
