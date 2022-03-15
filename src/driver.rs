use crate::protocol::{Packet, ParsePacketError, RawPacket, RAW_PACKET_LEN};
use crate::{dimX, dimY, Point};
use evdev_rs::enums::{BusType, EventCode, EventType, EV_ABS, EV_KEY, EV_SYN};
use evdev_rs::{
    AbsInfo, Device, DeviceWrapper, InputEvent, ReadFlag, TimeVal, UInputDevice, UninitDevice,
};
use input_linux::GenericEvent;
use nix::libc::time_t;
use std::time::{self, Duration, Instant, SystemTime};
use std::{error, fmt, io, thread};

const RIGHT_CLICK_THRESHOLD: Duration = Duration::from_millis(1500);

#[derive(Debug, PartialEq)]
struct Driver {
    touch_state: TouchState,
    monitor_cfg: MonitorConfig,
}

impl Driver {
    fn new() -> Self {
        Self {
            touch_state: TouchState::default(),
            monitor_cfg: MonitorConfig::default(),
        }
    }

    /// Update the internal state of the driver.
    /// Technically, Linux' input subsystem already filters out duplicate events so we could immediately turn the packet into InputEvent objects.
    /// But to support right clicks we must maintain some state.
    // TODO implement right-click if touching same spot (+- small area) for some amount of time
    fn update(&mut self, packet: Packet) -> Vec<ChangeSet> {
        let mut changes = Vec::new();

        match (self.touch_state.is_touching, packet.is_touching()) {
            (false, false) => {}
            (true, false) => {
                // self.touch_state.touch_start = None;
                changes.push(ChangeSet::Released);
                // if self.touch_state.is_right_click {
                //     self.touch_state.is_right_click = false;
                //     changes.push(ChangeSet::ReleasedRight);
                // }
            }
            (false, true) => {
                // self.touch_state.touch_start = Some(Instant::now());
                changes.push(ChangeSet::Pressed);
            }
            (true, true) => {
                // let touch_start = self.touch_state.touch_start.unwrap();
                // let time_touching = Instant::now().duration_since(touch_start);
                // if time_touching > RIGHT_CLICK_THRESHOLD && !self.touch_state.is_right_click {
                // self.touch_state.is_right_click = true;
                // changes.push(ChangeSet::PressedRight);
                // }
            }
        }
        self.touch_state.is_touching = packet.is_touching();

        if self.touch_state.x() != packet.x() {
            self.touch_state.set_x(packet.x());
            changes.push(ChangeSet::ChangedX(packet.x()));
        }

        if self.touch_state.y() != packet.y() {
            self.touch_state.set_y(packet.y());
            changes.push(ChangeSet::ChangedY(packet.y()));
        }

        changes
    }

    /// Setup the virtual device with uinput
    /// customized from evdev-rs' vmouse.rs
    fn get_virtual_device(&self) -> Result<UInputDevice, EgalaxError> {
        let u = UninitDevice::new().ok_or(EgalaxError::DeviceError)?;

        // Setup device
        // per: https://01.org/linuxgraphics/gfx-docs/drm/input/uinput.html#mouse-movements

        u.set_name("Egalax Virtual Mouse");
        u.set_bustype(BusType::BUS_USB as u16);
        u.set_vendor_id(0x0eef);
        u.set_product_id(0xcafe);

        u.enable_event_type(&EventType::EV_KEY)?;
        u.enable_event_code(&EventCode::EV_KEY(EV_KEY::BTN_TOUCH), None)?;
        // u.enable_event_code(&EventCode::EV_KEY(EV_KEY::BTN_RIGHT), None)?;

        let abs_info_x: AbsInfo = AbsInfo {
            value: 0,
            minimum: self.monitor_cfg.screen_space_ul.x.value().into(),
            maximum: self.monitor_cfg.screen_space_lr.x.value().into(),
            fuzz: 0,
            flat: 0,
            resolution: 0,
        };

        let abs_info_y: AbsInfo = AbsInfo {
            value: 0,
            minimum: self.monitor_cfg.screen_space_ul.y.value().into(),
            maximum: self.monitor_cfg.screen_space_lr.y.value().into(),
            fuzz: 0,
            flat: 0,
            resolution: 0,
        };

        u.enable_event_type(&EventType::EV_ABS)?;
        u.enable_event_code(&EventCode::EV_ABS(EV_ABS::ABS_X), Some(&abs_info_x))?;
        u.enable_event_code(&EventCode::EV_ABS(EV_ABS::ABS_Y), Some(&abs_info_y))?;

        // TODO do we need MSC_SCAN which is present in recording.txt?
        u.enable_event_code(&EventCode::EV_SYN(EV_SYN::SYN_REPORT), None)?;

        // Attempt to create UInputDevice from UninitDevice
        let vm = UInputDevice::create_from_device(&u).map_err(EgalaxError::IOError)?;

        // apparently you're supposed to sleep for a small amount of time so that udev can register the device
        thread::sleep(Duration::from_secs(1));
        Ok(vm)
    }

    fn send_event(&self, vm: &UInputDevice, changes: &[ChangeSet]) -> Result<(), EgalaxError> {
        // println!("Sending event {:#?}", changes);
        // let time = SystemTime::now()
        //     .try_into()
        //     .map_err(EgalaxError::TimeError)?;

        let ZERO = TimeVal::new(0, 0);

        for change in changes.iter() {
            let event = change.to_input_event(&self.monitor_cfg, &ZERO)?;
            vm.write_event(&event)?;
        }

        vm.write_event(&InputEvent {
            time: ZERO,
            event_code: EventCode::EV_SYN(EV_SYN::SYN_REPORT),
            value: 0,
        })?;

        Ok(())
    }
}

/// Changes for which we need to generate evdev events after we processed a packet
// TODO does it make sense to collapse ChangedX & ChangedY into a Changed(T, udim<T>)? Probably not possible
#[derive(Debug, PartialEq)]
enum ChangeSet {
    ChangedX(dimX),
    ChangedY(dimY),
    Pressed,
    Released,
    // PressedRight,
    // ReleasedRight,
}

impl ChangeSet {
    fn to_input_event(
        &self,
        monitor_cfg: &MonitorConfig,
        time: &TimeVal,
    ) -> Result<InputEvent, EgalaxError> {
        // TODO match self or *self. What's the difference?
        let (code, value) = match self {
            ChangeSet::Pressed => (EventCode::EV_KEY(EV_KEY::BTN_TOUCH), 1),
            ChangeSet::Released => (EventCode::EV_KEY(EV_KEY::BTN_TOUCH), 0),
            // ChangeSet::PressedRight => (EventCode::EV_KEY(EV_KEY::BTN_RIGHT), 1),
            // ChangeSet::ReleasedRight => (EventCode::EV_KEY(EV_KEY::BTN_RIGHT), 0),
            ChangeSet::ChangedX(x) => {
                let xn =
                    x.linear_factor(monitor_cfg.touch_event_ul.x, monitor_cfg.touch_event_lr.x);
                let xm = dimX::lerp(
                    monitor_cfg.monitor_area_ul.x,
                    monitor_cfg.monitor_area_lr.x,
                    xn,
                );
                (EventCode::EV_ABS(EV_ABS::ABS_X), xm.value())
            }
            ChangeSet::ChangedY(y) => {
                let yn =
                    y.linear_factor(monitor_cfg.touch_event_ul.y, monitor_cfg.touch_event_lr.y);
                let ym = dimY::lerp(
                    monitor_cfg.monitor_area_ul.y,
                    monitor_cfg.monitor_area_lr.y,
                    yn,
                );
                (EventCode::EV_ABS(EV_ABS::ABS_Y), ym.value())
            }
        };

        Ok(InputEvent::new(time, &code, value as i32))
    }
}

#[derive(Debug, PartialEq)]
struct TouchState {
    is_touching: bool,
    // is_right_click: bool,
    // touch_start: Option<Instant>,
    p: Point,
}

impl TouchState {
    pub fn is_touching(&self) -> bool {
        self.is_touching
    }

    pub fn x(&self) -> dimX {
        self.p.x
    }

    pub fn set_x(&mut self, x: dimX) -> () {
        self.p.x = x;
    }

    pub fn y(&self) -> dimY {
        self.p.y
    }

    pub fn set_y(&mut self, y: dimY) -> () {
        self.p.y = y;
    }
}

impl Default for TouchState {
    fn default() -> Self {
        TouchState {
            is_touching: false,
            // is_right_click: false,
            // touch_start: None,
            p: (0, 0).into(),
        }
    }
}

/// Parameters needed to translate the touch event coordinates coming from the monitor to coordinates in X's screen space.
/// a.d. TODO we might be able to remove some coordinates if we set the resolution in the uinput absinfo
#[derive(Debug, PartialEq)]
struct MonitorConfig {
    screen_space_ul: Point,
    screen_space_lr: Point,
    monitor_area_ul: Point,
    monitor_area_lr: Point,
    touch_event_ul: Point,
    touch_event_lr: Point,
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

#[derive(Debug)]
pub enum EgalaxError {
    UnexpectedEOF,
    DeviceError,
    TimeError(time::SystemTimeError),
    ParseError(ParsePacketError),
    IOError(io::Error),
}

impl From<io::Error> for EgalaxError {
    fn from(e: io::Error) -> Self {
        Self::IOError(e)
    }
}

impl From<ParsePacketError> for EgalaxError {
    fn from(e: ParsePacketError) -> Self {
        Self::ParseError(e)
    }
}

impl error::Error for EgalaxError {}

impl fmt::Display for EgalaxError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        // TODO match on self or *self?
        let description = match self {
            EgalaxError::ParseError(e) => return e.fmt(f),
            EgalaxError::IOError(e) => return e.fmt(f),
            EgalaxError::TimeError(e) => return e.fmt(f),
            EgalaxError::UnexpectedEOF => "Unexpected EOF",
            EgalaxError::DeviceError => "Device Error",
        };
        f.write_str(&description)
    }
}

/// Call a function on all packets in the given stream
pub fn process_packets<T, F>(stream: &mut T, f: &mut F) -> Result<(), EgalaxError>
where
    T: io::Read,
    F: FnMut(Packet) -> Result<(), EgalaxError>,
{
    let mut raw_packet: RawPacket = [0; RAW_PACKET_LEN];

    loop {
        // println!("read next packet");
        stream.read_exact(&mut raw_packet)?;
        let packet = Packet::try_from(raw_packet)?;
        f(packet)?;
    }
}

/// Print the sequence of packets in the given stream
pub fn print_packets(stream: &mut impl io::Read) -> Result<(), EgalaxError> {
    process_packets(stream, &mut |packet| Ok(println!("{:#?}", packet)))
}

/// Send evdev events for a virtual mouse based on the packets in the given stream
pub fn virtual_mouse(mut stream: impl io::Read) -> Result<(), EgalaxError> {
    let mut driver = Driver::new();
    let vm = driver.get_virtual_device()?;

    let mut process_packet = |packet| {
        // println!("processing packet");
        let changes = driver.update(packet);
        driver.send_event(&vm, &changes)
    };
    process_packets(&mut stream, &mut process_packet)
}
