use serde::{Deserialize, Serialize};
use std::fmt;
use std::fs::OpenOptions;
use std::path::Path;
use xrandr::{Monitor, XHandle};

use crate::{error::EgalaxError, geo::AABB};

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
    monitor_designator: MonitorDesignator,
    calibrated_area: Option<AABB>,
    /// The coordinates of the calibration points in the coordinate system of the touch screen (appears to be physically in units of 0.1mm).
    calibration_points: AABB,
}

impl MonitorConfigBuilder {
    pub fn new(
        monitor_designator: MonitorDesignator,
        calibrated_area: Option<AABB>,
        calibration_points: AABB,
    ) -> Self {
        MonitorConfigBuilder {
            monitor_designator,
            calibrated_area,
            calibration_points,
        }
    }

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

    fn compute_screen_space(&self, monitors: &[Monitor]) -> AABB {
        monitors
            .iter()
            .map(AABB::from)
            .fold(AABB::default(), AABB::union)
    }

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

        if let Some(calibrated_area) = self.calibrated_area {
            log::info!(
                "Using calibration {} to offset monitor dimensions ({}, {}).",
                calibrated_area,
                monitor.x,
                monitor.y
            );
            Ok(calibrated_area.shift(monitor.x, monitor.y))
        } else {
            let area = AABB::from(monitor);
            log::info!("Using uncalibrated monitor's total dimensions {}", area);
            Ok(area)
        }
    }
}

impl Default for MonitorConfigBuilder {
    fn default() -> Self {
        Self {
            monitor_designator: MonitorDesignator::Primary,
            calibrated_area: None,
            calibration_points: AABB::new(300, 300, 3800, 3800),
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
