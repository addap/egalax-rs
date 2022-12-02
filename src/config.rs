use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs::OpenOptions;
use std::path::Path;
use xrandr::{Monitor, XHandle};

use crate::{error::EgalaxError, geo::AABB};

/// Parameters needed to translate the touch event coordinates coming from the monitor to coordinates in X's screen space.
///
/// X has a virtual total screen space consisting of all connected displays. We have to move the mouse using absolute coordinates in this screen space.
/// Therefore, to compute the physical touch coordinates we need to know the calibration points of the touchscreen.
/// And to translate the phsyical touch coordinates into screen space coordinates we need to know the monitor area within the total screen space.
///
/// physical      screen space
/// +-----+      +-----+
/// |  A  |      |  A  +----+
/// |     | ---> |     | B  |
/// +-----+      +-----+----+
///   _+_
#[derive(Debug, PartialEq, Clone, Copy)]
pub struct MonitorConfig {
    /// Total virtual screen space in pixels. the union of all screen spaces of connected displays.
    pub screen_space: AABB,
    /// Screen space of the target monitor in absolute pixels.
    pub monitor_area: AABB,
    /// The coordinates of the calibration points in the coordinate system of the touch screen (appears to be physically in units of 0.1mm).
    pub calibration_points: AABB,
}

impl fmt::Display for MonitorConfig {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let description = format!("Total virtual screen space is {}.\nMonitor area within screen space is {}.\nCalibration points of touchscreen are {}", 
            self.screen_space,
            self.monitor_area,
            self.calibration_points);

        f.write_str(&description)
    }
}

/// Representation of config file which can be used to build a [MonitorConfig]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MonitorConfigBuilder {
    /// Name of the xrandr output of the monitor on which touch events will be interpreted.
    monitor_designator: MonitorDesignator,
    /// The coordinates of the calibration points in the coordinate system of the touch screen (appears to be physically in units of 0.1mm).
    calibration_points: AABB,
}

impl MonitorConfigBuilder {
    pub fn new(monitor_designator: MonitorDesignator, calibration_points: AABB) -> Self {
        MonitorConfigBuilder {
            monitor_designator,
            calibration_points,
        }
    }

    /// Load config from file.
    pub fn from_file<P>(path: P) -> Result<Self, EgalaxError>
    where
        P: AsRef<Path>,
    {
        log::debug!("Entering MonitorConfigBuilder::from_file");

        let f = OpenOptions::new().read(true).open(path)?;
        let config_file = serde_lexpr::from_reader(f)?;
        log::info!("Using config file '{:?}'", config_file);

        log::debug!("Leaving MonitorConfigBuilder::from_file");
        Ok(config_file)
    }

    /// Query info from Xrandr to build a [MonitorConfig].
    pub fn build(self) -> Result<MonitorConfig, EgalaxError> {
        log::debug!("Entering MonitorConfigBuilder::build");

        let monitors = XHandle::open()?.monitors()?;
        let screen_space = self.compute_screen_space(&monitors);
        let monitor_area = self.get_monitor_area(&monitors)?;

        log::debug!("Leaving MonitorConfigBuilder::build");
        Ok(MonitorConfig {
            screen_space: screen_space,
            monitor_area: monitor_area,
            calibration_points: self.calibration_points,
        })
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

impl Default for MonitorConfigBuilder {
    fn default() -> Self {
        Self {
            monitor_designator: MonitorDesignator::Primary,
            calibration_points: AABB::from((300, 300, 3800, 3800)),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MonitorDesignator {
    Primary,
    Named(String),
}

impl ToString for MonitorDesignator {
    fn to_string(&self) -> String {
        match self {
            MonitorDesignator::Primary => String::from("Primary"),
            MonitorDesignator::Named(name) => name.clone(),
        }
    }
}
