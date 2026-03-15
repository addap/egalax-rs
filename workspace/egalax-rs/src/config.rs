use serde::{Deserialize, Serialize};
use std::{fmt, str::FromStr};

use crate::{
    error::{ConfigParseError, ConfigSerializeError},
    geo::AABB,
};

// a.d. TODO use configparser instead of serde.
/// Common config options that are taken verbatim from the config file.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Config {
    // a.d. TODO make optional
    /// The coordinates of the calibration points in the coordinate system of the touch screen (appears to be physically in units of 0.1mm).
    pub calibration_points: AABB,
}

impl Config {
    pub fn calibration_points(&self) -> AABB {
        self.calibration_points
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            calibration_points: AABB::from((300, 300, 3800, 3800)),
        }
    }
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // a.d. Note that the backslash in a string literal escapes both the line break and the leasing whitespace of the next line.
        write!(
            f,
            "Calibration points of touchscreen: {}.",
            self.calibration_points,
        )
    }
}

impl FromStr for Config {
    type Err = ConfigParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(toml::from_str(s)?)
    }
}

impl Config {
    /// Serialize config in TOML format.
    pub fn to_toml_string(&self) -> Result<String, ConfigSerializeError> {
        Ok(toml::to_string_pretty(&self)?)
    }
}
