use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs::OpenOptions;
use std::path::Path;
use xrandr::{Monitor, XHandle};

use crate::{driver::EgalaxError, geo::AABB};

/// Parameters needed to translate the touch event coordinates coming from the monitor to coordinates in X's screen space.
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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MonitorConfigBuilder {
    /// Name of the xrandr output of the monitor on which touch events will be interpreted.
    monitor_area_designator: MonitorAreaDesignator,
    /// The coordinates of the calibration points in the coordinate system of the touch screen (appears to be physically in units of 0.1mm).
    calibration_points: AABB,
}

impl MonitorConfigBuilder {
    pub fn new(monitor_area_designator: MonitorAreaDesignator, calibration_points: AABB) -> Self {
        MonitorConfigBuilder {
            monitor_area_designator,
            calibration_points,
        }
    }

    pub fn from_file<P>(path: P) -> Result<Self, EgalaxError>
    where
        P: AsRef<Path>,
    {
        let f = OpenOptions::new().read(true).open(path)?;
        let config_file = serde_lexpr::from_reader(f)?;
        Ok(config_file)
    }

    pub fn with_name(mut self, monitor_name: String) -> Self {
        self.monitor_area_designator = MonitorAreaDesignator::Name(monitor_name);
        self
    }

    pub fn build(self) -> Result<MonitorConfig, EgalaxError> {
        let monitors = XHandle::open()?.monitors()?;
        let screen_space = self.compute_screen_space(&monitors);
        let monitor_area = self.get_monitor_area(&monitors)?;

        Ok(MonitorConfig {
            screen_space: screen_space,
            monitor_area: monitor_area,
            calibration_points: self.calibration_points,
        })
    }

    fn compute_screen_space(&self, monitors: &[Monitor]) -> AABB {
        monitors
            .iter()
            .map(AABB::from)
            .fold(AABB::default(), AABB::union)
    }

    fn get_monitor_area(&self, monitors: &[Monitor]) -> Result<AABB, EgalaxError> {
        // If we have a name we look for a monitor with that name
        // otherwise we just take the primary monitor, which must exist.
        match &self.monitor_area_designator {
            MonitorAreaDesignator::Primary => {
                let primary = monitors.iter().find(|monitor| monitor.is_primary).unwrap();
                Ok(AABB::from(primary))
            }
            MonitorAreaDesignator::Name(monitor_name) => monitors
                .iter()
                .find_map(|monitor| {
                    if monitor.name == *monitor_name {
                        Some(AABB::from(monitor))
                    } else {
                        None
                    }
                })
                .ok_or(EgalaxError::MonitorNotFound(monitor_name.clone())),
            MonitorAreaDesignator::Area(monitor_area) => Ok(*monitor_area),
        }
    }
}

impl Default for MonitorConfigBuilder {
    fn default() -> Self {
        Self {
            monitor_area_designator: MonitorAreaDesignator::Primary,
            calibration_points: AABB::new(300, 300, 3800, 3800),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum MonitorAreaDesignator {
    Primary,
    Name(String),
    Area(AABB),
}
