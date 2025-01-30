use evdev_rs::enums::EV_KEY;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs;
use std::path::Path;
use std::time::Duration;
use xrandr::Monitor;

use crate::{error::EgalaxError, geo::AABB};

/// Parameters needed to translate the touch event coordinates coming from the monitor to coordinates in X's screen space.
///
/// X has a virtual total screen space consisting of all connected displays. We have to move the mouse using absolute coordinates in this screen space.
/// Therefore, to compute the physical touch coordinates we need to know the calibration points of the touchscreen.
/// And to translate the phsyical touch coordinates into screen space coordinates we need to know the monitor area within the total screen space.
///
/// physical            screen space
/// +-----+             +-----+----+ (upper right area exists in virtual screen space
/// |  A  | +----+      |  A  +----+   but cursor cannot move there.)
/// |     | | B  | ---> |     | B  |
/// +-----+ +----+      +-----+----+
///    |      |
///   _+_    _+_
#[derive(Debug, Clone, Copy)]
pub struct Config {
    /// Total virtual screen space in pixels. the union of all screen spaces of connected displays.
    pub screen_space: AABB,
    /// Screen space of the target monitor in absolute pixels.
    pub monitor_area: AABB,
    /// Common config options.
    common: ConfigCommon,
}

impl Config {
    pub fn calibration_points(&self) -> AABB {
        self.common.calibration_points
    }

    pub fn right_click_wait(&self) -> Duration {
        self.common.right_click_wait()
    }

    pub fn has_moved_threshold(&self) -> f32 {
        self.common.has_moved_threshold
    }

    pub fn ev_left_click(&self) -> EV_KEY {
        self.common.ev_left_click
    }

    pub fn ev_right_click(&self) -> EV_KEY {
        self.common.ev_right_click
    }
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_fmt(format_args!(
            "Total virtual screen space: {}.\n\
            Monitor area within screen space: {}.
            {}",
            self.screen_space, self.monitor_area, self.common
        ))
    }
}

// TODO use configparser instead of serde.
/// Common config options that are taken verbatim from the config file.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ConfigCommon {
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

impl ConfigCommon {
    pub fn right_click_wait(&self) -> Duration {
        Duration::from_millis(self.right_click_wait_ms)
    }
}

impl Default for ConfigCommon {
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

impl fmt::Display for ConfigCommon {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!(
            "Calibration points of touchscreen: {}.\n\
            Right-click wait duration: {}ms.\n\
            Has-moved threshold: {}mm.",
            self.calibration_points,
            self.right_click_wait_ms,
            self.has_moved_threshold * 0.1,
        ))
    }
}

/// Representation of config file which can be used to build a [MonitorConfig]
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ConfigFile {
    /// Name of the xrandr output of the monitor on which touch events will be interpreted.
    pub monitor_designator: MonitorDesignator,
    /// Common config options.
    pub common: ConfigCommon,
}

impl ConfigFile {
    pub fn new(monitor_designator: MonitorDesignator, common: ConfigCommon) -> Self {
        Self {
            monitor_designator,
            common,
        }
    }

    /// Load config from file.
    pub fn from_file<P>(path: P) -> Result<Self, EgalaxError>
    where
        P: AsRef<Path>,
    {
        log::trace!("Entering ConfigFile::from_file.");

        let config_file = fs::read_to_string(path)?;
        let config_file = toml::from_str(&config_file)?;
        log::debug!("Loaded config file:\n{}", config_file);

        log::trace!("Leaving ConfigFile::from_file.");
        Ok(config_file)
    }

    pub fn save_file<P>(&self, path: P) -> Result<(), EgalaxError>
    where
        P: AsRef<Path>,
    {
        log::trace!("Entering ConfigFile::save_file");

        let config_file = toml::to_string_pretty(&self)?;
        log::debug!("Saving config file:\n{}", config_file);
        fs::write(path, config_file)?;

        log::trace!("Leaving ConfigFile::save_file");
        Ok(())
    }

    /// Query info from Xrandr to build a [MonitorConfig].
    pub fn build(self) -> Result<Config, EgalaxError> {
        log::trace!("Entering MonitorConfigBuilder::build");

        let monitors = xrandr::XHandle::open()?.monitors()?;
        let screen_space = self.compute_screen_space(&monitors);
        let monitor_area = self.get_monitor_area(&monitors)?;

        let config = Config {
            screen_space: screen_space,
            monitor_area: monitor_area,
            common: self.common,
        };
        log::trace!("Leaving MonitorConfigBuilder::build");
        Ok(config)
    }

    /// Union screen spaces of all monitors to get total screen space used by X.
    fn compute_screen_space(&self, monitors: &[Monitor]) -> AABB {
        monitors
            .iter()
            .map(AABB::from)
            .fold(AABB::default(), AABB::union)
    }

    /// Get only the screen space of the touchscreen monitor.
    fn get_monitor_area(&self, monitors: &[Monitor]) -> Result<AABB, EgalaxError> {
        let monitor = match &self.monitor_designator {
            MonitorDesignator::Primary => monitors.iter().find(|monitor| monitor.is_primary),
            MonitorDesignator::Named(monitor_name) => monitors
                .iter()
                .find(|monitor| monitor.name == *monitor_name),
        }
        .ok_or(EgalaxError::MonitorNotFound(
            self.monitor_designator.to_string(),
        ))?;

        let area = AABB::from(monitor);
        log::info!("Using uncalibrated monitor's total dimensions {}", area);
        Ok(area)
    }
}

impl fmt::Display for ConfigFile {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let description = format!(
            "Name of XRandR Output: {}.\n{}",
            self.monitor_designator, self.common
        );

        f.write_str(&description)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MonitorDesignator {
    Primary,
    Named(String),
}

impl fmt::Display for MonitorDesignator {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let description = match self {
            MonitorDesignator::Primary => String::from("*Primary*"),
            MonitorDesignator::Named(name) => name.clone(),
        };
        f.write_str(&description)
    }
}

impl Default for MonitorDesignator {
    fn default() -> Self {
        Self::Primary
    }
}
