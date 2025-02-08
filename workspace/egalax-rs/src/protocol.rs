//! Implements parsing of the packets that are received from the hidraw interface.

use evdev_rs::TimeVal;
use std::fmt;

#[allow(clippy::wildcard_imports)]
use crate::{error::ParsePacketError, geo::Point2D, units::*};

/// Length of a raw packet.
pub const RAW_PACKET_LEN: usize = 6;

/// Type of raw packets.
#[derive(Debug, Clone, Copy)]
pub struct RawPacket(pub [u8; RAW_PACKET_LEN]);

impl fmt::Display for RawPacket {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // The #04x format specifier pads the value with 0's to a length of 4, prepends '0x' (leaving a length of 2 for the number itself)
        // and shows the value in upper-case hex format.
        // from https://doc.rust-lang.org/std/fmt/#sign0
        write!(
            f,
            "[{:#04X}, {:#04X}, {:#04X}, {:#04X}, {:#04X}, {:#04X}]",
            self.0[0], self.0[1], self.0[2], self.0[3], self.0[4], self.0[5]
        )
    }
}

/// A boolean indicating if a finger touch is detected.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TouchState {
    IsTouching,
    NotTouching,
}

/// Type of packet tags that we currently support.
#[repr(u8)]
pub enum PacketTag {
    TouchEvent = 0x2,
}

/// A representation of a packet sent over USB.
/// If we support more message types this should be extended to an enum with different packet variants.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct USBPacket {
    touch_state: TouchState,
    position: Point2D,
    resolution: u8,
}

impl USBPacket {
    pub fn with_time(self, time: TimeVal) -> USBMessage {
        USBMessage { time, packet: self }
    }

    pub fn touch_state(&self) -> TouchState {
        self.touch_state
    }

    pub fn position(&self) -> Point2D {
        self.position
    }

    pub fn resolution(&self) -> u8 {
        self.resolution
    }

    /// Parsing logic for a touch event packet.
    /// Fails if the package is somehow malformed.
    pub fn try_parse(
        packet: RawPacket,
        expected_tag: Option<PacketTag>,
    ) -> Result<Self, ParsePacketError> {
        // Bitmasks for fields in the raw packet.
        const TOUCH_STATE_MASK: u8 = 0x01;
        const RESOLUTION_MASK: u8 = 0x06;

        log::trace!("Entering Packet::try_parse.");

        if let Some(expected_tag) = expected_tag {
            let raw_tag = packet.0[0];
            if raw_tag != expected_tag as u8 {
                return Err(ParsePacketError::UnexpectedTag(raw_tag));
            }
        }

        let resolution = match packet.0[1] & RESOLUTION_MASK {
            0x00 => 11,
            0x02 => 12,
            0x04 => 13,
            0x05 => 14,
            _ => unreachable!("Only two bits should be left, match can never succeed"),
        };

        let touch_state = if (packet.0[1] & TOUCH_STATE_MASK) == 0x01 {
            TouchState::IsTouching
        } else {
            TouchState::NotTouching
        };

        // X and Y coordinates are stored little-endian.
        let y = (u16::from(packet.0[3]) << 8) | u16::from(packet.0[2]);
        let x = (u16::from(packet.0[5]) << 8) | u16::from(packet.0[4]);

        if y >> resolution != 0x00 {
            return Err(ParsePacketError::WrongResolution(DimE::Y));
        } else if x >> resolution != 0x00 {
            return Err(ParsePacketError::WrongResolution(DimE::X));
        }

        let packet = USBPacket {
            touch_state,
            position: Point2D {
                x: x.into(),
                y: y.into(),
            },
            resolution,
        };

        log::trace!("Leaving Packet::try_parse.");
        Ok(packet)
    }
}

impl fmt::Display for USBPacket {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let touch = match self.touch_state {
            TouchState::IsTouching => "1",
            TouchState::NotTouching => "0",
        };
        write!(f, "Touch={}, Point={}", touch, self.position)
    }
}

/// Messages are timestamped to give them to evdev later.
#[derive(Debug, Clone, Copy)]
pub struct USBMessage {
    time: TimeVal,
    packet: USBPacket,
}

impl USBMessage {
    pub fn time(&self) -> TimeVal {
        self.time
    }

    pub fn packet(&self) -> &USBPacket {
        &self.packet
    }
}

impl fmt::Display for USBMessage {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Message at {:?}\nPacket: {}", self.time, self.packet)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_touch_upper_left() {
        let raw_packet: RawPacket = RawPacket([0x02, 0x03, 0x3b, 0x01, 0x32, 0x01]);

        assert_eq!(
            Ok(USBPacket {
                touch_state: TouchState::IsTouching,
                position: (306, 315).into(),
                resolution: 12
            }),
            USBPacket::try_parse(raw_packet, Some(PacketTag::TouchEvent))
        );
    }

    #[test]
    fn test_parse_release_upper_left() {
        let raw_packet: RawPacket = RawPacket([0x02, 0x02, 0x35, 0x01, 0x39, 0x01]);

        assert_eq!(
            Ok(USBPacket {
                touch_state: TouchState::IsTouching,
                position: (313, 309).into(),
                resolution: 12
            }),
            USBPacket::try_parse(raw_packet, Some(PacketTag::TouchEvent))
        );
    }

    #[test]
    fn test_malformed_const() {
        let raw_packet: RawPacket = RawPacket([0xaa, 0x02, 0x35, 0x01, 0x39, 0x01]);

        assert_eq!(
            Err(ParsePacketError::UnexpectedTag(0xaa)),
            USBPacket::try_parse(raw_packet, Some(PacketTag::TouchEvent))
        );
    }

    #[test]
    fn test_malformed_res_y() {
        let raw_packet: RawPacket = RawPacket([0x02, 0x02, 0x35, 0x11, 0x39, 0x01]);

        assert_eq!(
            Err(ParsePacketError::WrongResolution(DimE::Y)),
            USBPacket::try_parse(raw_packet, Some(PacketTag::TouchEvent))
        );
    }

    #[test]
    fn test_malformed_res_x() {
        let raw_packet: RawPacket = RawPacket([0x02, 0x02, 0x35, 0x01, 0x39, 0x11]);

        assert_eq!(
            Err(ParsePacketError::WrongResolution(DimE::X)),
            USBPacket::try_parse(raw_packet, Some(PacketTag::TouchEvent))
        );
    }
}
