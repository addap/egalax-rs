use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use std::fs::OpenOptions;
use std::path::Path;
use std::time::Duration;
use std::{fmt, io::Read};
use xrandr::{Monitor, XHandle};

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
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct Config {
    /// Total virtual screen space in pixels. the union of all screen spaces of connected displays.
    pub screen_space: AABB,
    /// Screen space of the target monitor in absolute pixels.
    pub monitor_area: AABB,
    /// The coordinates of the calibration points in the coordinate system of the touch screen (appears to be physically in units of 0.1mm).
    pub calibration_points: AABB,
    /// How long you have to keep pressing to trigger a right-click.
    pub right_click_wait: Duration,
    /// Threshold to filter noise of consecutive touch events happening close to each other.
    pub has_moved_threshold: f64,
}

impl fmt::Display for Config {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let description = format!(
            "Total virtual screen space: {}.
            Monitor area within screen space: {}.
            Calibration points of touchscreen: {}.
            Right-click wait duration: {}ms.
            Has-moved threshold: {}.",
            self.screen_space,
            self.monitor_area,
            self.calibration_points,
            self.right_click_wait.as_millis(),
            self.has_moved_threshold
        );

        f.write_str(&description)
    }
}

/// Representation of config file which can be used to build a [MonitorConfig]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfigFile {
    /// Name of the xrandr output of the monitor on which touch events will be interpreted.
    monitor_designator: MonitorDesignator,
    /// The coordinates of the calibration points in the coordinate system of the touch screen (appears to be physically in units of 0.1mm).
    calibration_points: AABB,
    /// How long you have to keep pressing to trigger a right-click.
    right_click_wait: Duration,
    /// Threshold to filter noise of consecutive touch events happening close to each other.
    has_moved_threshold: f64,
}

impl ConfigFile {
    /// Load config from file.
    pub fn from_file<P>(path: P) -> Result<Self, EgalaxError>
    where
        P: AsRef<Path>,
    {
        log::trace!("Entering MonitorConfigBuilder::from_file");

        let mut f = OpenOptions::new().read(true).open(path)?;
        let mut config_file = String::new();
        f.read_to_string(&mut config_file)?;
        let config_file = toml::from_str(&config_file).map_err(|e| anyhow!(e))?;
        log::debug!("Using config file:\n{}", config_file);

        log::trace!("Leaving MonitorConfigBuilder::from_file");
        Ok(config_file)
    }

    /// Query info from Xrandr to build a [MonitorConfig].
    pub fn build(self) -> Result<Config, EgalaxError> {
        log::trace!("Entering MonitorConfigBuilder::build");

        let monitors = XHandle::open()?.monitors()?;
        let screen_space = self.compute_screen_space(&monitors);
        let monitor_area = self.get_monitor_area(&monitors)?;

        let config = Config {
            screen_space: screen_space,
            monitor_area: monitor_area,
            calibration_points: self.calibration_points,
            right_click_wait: self.right_click_wait,
            has_moved_threshold: self.has_moved_threshold,
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

impl Default for ConfigFile {
    fn default() -> Self {
        Self {
            monitor_designator: MonitorDesignator::Named("HDMI-A-0".to_string()),
            calibration_points: AABB::from((300, 300, 3800, 3800)),
            right_click_wait: Duration::from_millis(1500),
            has_moved_threshold: 30.0,
        }
    }
}

impl fmt::Display for ConfigFile {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let description = format!(
            "Name of XRandR Output: {}.\nCalibration points of touchscreen: {}",
            self.monitor_designator, self.calibration_points
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
            MonitorDesignator::Primary => String::from("Primary"),
            MonitorDesignator::Named(name) => name.clone(),
        };
        f.write_str(&description)
    }
}
