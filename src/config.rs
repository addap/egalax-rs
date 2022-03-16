use crate::driver::EgalaxError;
use crate::Point;
use std::{
    cmp::{max, min},
    error,
    ops::Add,
};
use xrandr::{Monitor, XHandle};

pub struct MonitorConfigBuilder {
    name: Option<String>,
    monitors: Vec<Monitor>,
}

impl MonitorConfigBuilder {
    pub fn new() -> Result<Self, EgalaxError> {
        let monitors = XHandle::open()?.monitors()?;
        Ok(MonitorConfigBuilder {
            name: None,
            monitors,
        })
    }

    pub fn with_name(mut self, name: Option<String>) -> Self {
        self.name = name;
        self
    }

    pub fn build(self) -> Result<MonitorConfig, EgalaxError> {
        let screen_space = self.compute_screen_space();
        let monitor_area = self.get_monitor_area()?;

        Ok(MonitorConfig {
            screen_space_ul: (screen_space.x1, screen_space.y1).into(),
            screen_space_lr: (screen_space.x2, screen_space.y2).into(),
            monitor_area_ul: (monitor_area.x1, monitor_area.y1).into(),
            monitor_area_lr: (monitor_area.x2, monitor_area.y2).into(),
            // TODO should be able to query from monitor
            touch_event_ul: (300, 300).into(),
            touch_event_lr: (3800, 3800).into(),
        })
    }

    fn compute_screen_space(&self) -> AABB {
        self.monitors
            .iter()
            .map(AABB::from)
            .fold(AABB::new(), <AABB as Add>::add)
    }

    fn get_monitor_area(&self) -> Result<AABB, EgalaxError> {
        // If we have a name we look for a monitor with that name
        // otherwise we just take the primary monitor, which must exist.
        if let Some(name) = &self.name {
            self.monitors
                .iter()
                .find_map(|monitor| {
                    if monitor.name == *name {
                        Some(AABB::from(monitor))
                    } else {
                        None
                    }
                })
                .ok_or(EgalaxError::MonitorNotFound(name.clone()))
        } else {
            let primary = self
                .monitors
                .iter()
                .find(|monitor| monitor.is_primary)
                .unwrap();
            Ok(AABB::from(primary))
        }
    }
}

/// Parameters needed to translate the touch event coordinates coming from the monitor to coordinates in X's screen space.
/// a.d. TODO we might be able to remove some coordinates if we set the resolution in the uinput absinfo
#[derive(Debug, PartialEq)]
pub struct MonitorConfig {
    pub screen_space_ul: Point,
    pub screen_space_lr: Point,
    pub monitor_area_ul: Point,
    pub monitor_area_lr: Point,
    pub touch_event_ul: Point,
    pub touch_event_lr: Point,
}

// TODO need to get monitor dimensions from xrandr or config file
impl Default for MonitorConfig {
    fn default() -> Self {
        MonitorConfig {
            screen_space_ul: (0, 0).into(),
            screen_space_lr: (3200, 1080).into(),
            monitor_area_ul: (1920, 0).into(),
            monitor_area_lr: (3200, 1024).into(),
            touch_event_ul: (300, 300).into(),
            touch_event_lr: (3800, 3800).into(),
        }
    }
}

/// An axis-aligned bounding box consisting of an upper left corner (x1, y1) and lower right corner (x2, y2)
#[derive(Debug, PartialEq)]
struct AABB {
    x1: i32,
    y1: i32,
    x2: i32,
    y2: i32,
}

impl AABB {
    fn new() -> Self {
        AABB {
            x1: 0,
            y1: 0,
            x2: 0,
            y2: 0,
        }
    }
}

impl Add for AABB {
    type Output = AABB;

    fn add(self, rhs: Self) -> Self::Output {
        AABB {
            x1: min(self.x1, rhs.x1),
            y1: min(self.y1, rhs.y1),
            x2: max(self.x2, rhs.x2),
            y2: max(self.y2, rhs.y2),
        }
    }
}

impl From<&xrandr::Monitor> for AABB {
    fn from(m: &xrandr::Monitor) -> Self {
        AABB {
            x1: m.x,
            y1: m.y,
            x2: m.x + m.width_px,
            y2: m.y + m.height_px,
        }
    }
}
