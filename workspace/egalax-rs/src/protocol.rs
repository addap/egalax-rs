//! Implements parsing of the HID reports that are received from the hidraw interface.

use std::fmt;

#[allow(clippy::wildcard_imports)]
use crate::{error::ParseReportError, geo::Point2D, units::*};

/// Length of a numbered report.
pub const NUMBERED_REPORT_LEN: usize = 6;

/// Type of raw HID reports.
#[derive(Debug, Clone, Copy)]
pub struct RawNumberedReport(pub [u8; NUMBERED_REPORT_LEN]);

impl fmt::Display for RawNumberedReport {
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

/// HID report numbers which our monitor can generate as per the HID report descriptor.
/// From observation it seems to only send [`Stylus`] reports.
// TODO use `unit_enum` crate?
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ReportNum {
    Pointer = 0x1,
    Stylus = 0x2,
}

impl TryFrom<u8> for ReportNum {
    type Error = ParseReportError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            0x01 => Ok(ReportNum::Pointer),
            0x02 => Ok(ReportNum::Stylus),
            _ => Err(ParseReportError::UnexpectedReportNum(value)),
        }
    }
}

impl fmt::Display for ReportNum {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "#{}", *self as u8)
    }
}

/// Both HID reports that our monitor advertises contain this data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Report {
    touch_state: TouchState,
    position: Point2D,
    resolution: u8,
}

impl Report {
    pub fn touch_state(&self) -> TouchState {
        self.touch_state
    }

    pub fn position(&self) -> Point2D {
        self.position
    }

    pub fn resolution(&self) -> u8 {
        self.resolution
    }

    /// Parsing logic for a touch event report.
    /// Fails if the package is somehow malformed.
    pub fn try_parse(
        raw_report: RawNumberedReport,
        expected_report_num: Option<ReportNum>,
    ) -> Result<Self, ParseReportError> {
        // Bitmasks for fields in the raw report.
        const TOUCH_STATE_MASK: u8 = 0x01;
        const RESOLUTION_MASK: u8 = 0x06;

        log::trace!("Entering Report::try_parse.");

        let raw_report_num = ReportNum::try_from(raw_report.0[0])?;
        if let Some(expected_report_num) = expected_report_num {
            if raw_report_num != expected_report_num {
                log::warn!(
                    "Unexpected report number: {}. Expected: {}",
                    raw_report_num,
                    expected_report_num as u8
                );
            }
        }

        let resolution = match raw_report.0[1] & RESOLUTION_MASK {
            0x00 => 11,
            0x02 => 12,
            0x04 => 13,
            0x05 => 14,
            _ => unreachable!("Only two bits should be left, match can never succeed"),
        };

        let touch_state = if (raw_report.0[1] & TOUCH_STATE_MASK) == 0x01 {
            TouchState::IsTouching
        } else {
            TouchState::NotTouching
        };

        // X and Y coordinates are stored little-endian.
        let y = (u16::from(raw_report.0[3]) << 8) | u16::from(raw_report.0[2]);
        let x = (u16::from(raw_report.0[5]) << 8) | u16::from(raw_report.0[4]);

        if y >> resolution != 0x00 {
            return Err(ParseReportError::WrongResolution(DimE::Y));
        } else if x >> resolution != 0x00 {
            return Err(ParseReportError::WrongResolution(DimE::X));
        }

        let report = Report {
            touch_state,
            position: Point2D {
                x: x.into(),
                y: y.into(),
            },
            resolution,
        };

        log::trace!("Leaving Report::try_parse.");
        Ok(report)
    }
}

impl fmt::Display for Report {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let touch = match self.touch_state {
            TouchState::IsTouching => "1",
            TouchState::NotTouching => "0",
        };
        write!(f, "Touch={}, Point={}", touch, self.position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_touch_upper_left() {
        let raw_report: RawNumberedReport = RawNumberedReport([0x02, 0x03, 0x3b, 0x01, 0x32, 0x01]);

        assert_eq!(
            Ok(Report {
                touch_state: TouchState::IsTouching,
                position: (306, 315).into(),
                resolution: 12
            }),
            Report::try_parse(raw_report, Some(ReportNum::Stylus))
        );
    }

    #[test]
    fn test_parse_release_upper_left() {
        let raw_report: RawNumberedReport = RawNumberedReport([0x02, 0x02, 0x35, 0x01, 0x39, 0x01]);

        assert_eq!(
            Ok(Report {
                touch_state: TouchState::IsTouching,
                position: (313, 309).into(),
                resolution: 12
            }),
            Report::try_parse(raw_report, Some(ReportNum::Stylus))
        );
    }

    #[test]
    fn test_malformed_const() {
        let raw_report: RawNumberedReport = RawNumberedReport([0xaa, 0x02, 0x35, 0x01, 0x39, 0x01]);

        assert_eq!(
            Err(ParseReportError::UnexpectedReportNum(0xaa)),
            Report::try_parse(raw_report, Some(ReportNum::Stylus))
        );
    }

    #[test]
    fn test_malformed_res_y() {
        let raw_report: RawNumberedReport = RawNumberedReport([0x02, 0x02, 0x35, 0x11, 0x39, 0x01]);

        assert_eq!(
            Err(ParseReportError::WrongResolution(DimE::Y)),
            Report::try_parse(raw_report, Some(ReportNum::Stylus))
        );
    }

    #[test]
    fn test_malformed_res_x() {
        let raw_report: RawNumberedReport = RawNumberedReport([0x02, 0x02, 0x35, 0x01, 0x39, 0x11]);

        assert_eq!(
            Err(ParseReportError::WrongResolution(DimE::X)),
            Report::try_parse(raw_report, Some(ReportNum::Stylus))
        );
    }
}
