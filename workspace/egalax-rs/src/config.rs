use evdev_rs::enums::EV_KEY;
use serde::{Deserialize, Serialize};
use std::{fmt, fs::File, io::Read, io::Write, time::Duration};

use crate::{error::EgalaxError, geo::AABB};

// a.d. TODO use configparser instead of serde.
/// Common config options that are taken verbatim from the config file.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Config {
    // a.d. TODO make optional
    /// The coordinates of the calibration points in the coordinate system of the touch screen (appears to be physically in units of 0.1mm).
    pub calibration_points: AABB,
    /// How long you have to keep pressing to trigger a right-click.
    pub right_click_wait_ms: u64,
    /// Threshold to filter noise of consecutive touch events happening close to each other.
    pub has_moved_threshold: f32,
    /// Key code for left-click.
    pub ev_left_click: EV_KEY,
    /// Key code for right-click.
    pub ev_right_click: EV_KEY,
}

impl Config {
    pub fn calibration_points(&self) -> AABB {
        self.calibration_points
    }

    pub fn right_click_wait(&self) -> Duration {
        Duration::from_millis(self.right_click_wait_ms)
    }

    pub fn has_moved_threshold(&self) -> f32 {
        self.has_moved_threshold
    }

    pub fn ev_left_click(&self) -> EV_KEY {
        self.ev_left_click
    }

    pub fn ev_right_click(&self) -> EV_KEY {
        self.ev_right_click
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            calibration_points: AABB::from((300, 300, 3800, 3800)),
            right_click_wait_ms: 1500,
            has_moved_threshold: 30.0,
            ev_left_click: EV_KEY::BTN_LEFT,
            ev_right_click: EV_KEY::BTN_RIGHT,
        }
    }
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // a.d. Note that the backslash in a string literal escapes both the line break and the leasing whitespace of the next line.
        write!(
            f,
            "Calibration points of touchscreen: {}.\n\
            Right-click wait duration: {}ms.\n\
            Has-moved threshold: {}mm.",
            self.calibration_points,
            self.right_click_wait_ms,
            self.has_moved_threshold * 0.1,
        )
    }
}

impl Config {
    /// Load config from file.
    pub fn from_file(f: &mut File) -> Result<Self, EgalaxError> {
        log::trace!("Entering Config::from_file.");

        let mut config_file = String::new();
        f.read_to_string(&mut config_file)?;
        let config = toml::from_str(&config_file)?;

        log::trace!("Leaving Config::from_file.");
        Ok(config)
    }

    pub fn to_toml_string(&self) -> Result<String, EgalaxError> {
        Ok(toml::to_string_pretty(&self)?)
    }

    /// Save config to file.
    pub fn save_file(&self, f: &mut File) -> Result<(), EgalaxError> {
        log::trace!("Entering Config::save_file");

        let config_file = self.to_toml_string()?;
        f.write_all(config_file.as_bytes())?;

        log::trace!("Leaving Config::save_file");
        Ok(())
    }
}
