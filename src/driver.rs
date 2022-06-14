use crate::config::MonitorConfig;
use crate::protocol::{
    Packet, ParsePacketError, RawPacket,
    TouchState::{self, *},
    RAW_PACKET_LEN,
};
use crate::{geo::Point, units::*};
use evdev_rs::enums::{BusType, EventCode, EventType, InputProp, EV_ABS, EV_KEY, EV_SYN};
use evdev_rs::{AbsInfo, DeviceWrapper, InputEvent, TimeVal, UInputDevice, UninitDevice};
use std::time::{self, Duration, Instant, SystemTime};
use std::{error, fmt, io, thread};

// TODO test values for has_moved thresh
const HAS_MOVED_THRESHOLD: f64 = 30.0;
const RIGHT_CLICK_THRESHOLD: Duration = Duration::from_millis(1500);
const BTN_LEFT: EV_KEY = EV_KEY::BTN_TOUCH;
const BTN_RIGHT: EV_KEY = EV_KEY::BTN_STYLUS2;

#[derive(Debug, PartialEq)]
struct Driver {
    state: DriverState,
    monitor_cfg: MonitorConfig,
}

impl Driver {
    fn new(monitor_cfg: MonitorConfig) -> Self {
        Self {
            state: DriverState::default(),
            monitor_cfg,
        }
    }

    /// Update the internal state of the driver.
    /// Technically, Linux' input subsystem already filters out duplicate events so we could immediately turn the packet into InputEvent objects.
    /// But to support right clicks we must maintain some state.
    fn update(&mut self, packet: Packet) -> Vec<InputEvent> {
        log::info!("Processing packet: {}", packet);

        let mut events = EventGen::new(packet.time());

        // Compare last with current touch state
        match (self.state.touch_state, packet.touch_state()) {
            (NotTouching, NotTouching) => {}
            (IsTouching, NotTouching) => {
                log::info!("Releasing left-click.");
                events.emit_btn_release(BTN_LEFT);

                if self.state.is_right_click {
                    log::info!("Releasing right-click.");
                    events.emit_btn_release(BTN_RIGHT);
                }
                self.state.reset();
            }
            (NotTouching, IsTouching) => {
                log::info!("Starting touch.");
                self.state.touch_start_time = Some(Instant::now());
                self.state.touch_origin = Some(Point::from((packet.x(), packet.y())));
                events.emit_btn_press(BTN_LEFT);
            }
            (IsTouching, IsTouching) => {
                if !self.state.is_right_click && !self.state.has_moved {
                    // check if during press we moved too far away from origin and diable right-click
                    let touch_origin = self.state.touch_origin.as_ref().unwrap();
                    let touch_distance =
                        touch_origin.euc_distance_to(&Point::from((packet.x(), packet.y())));

                    if touch_distance > HAS_MOVED_THRESHOLD {
                        log::info!("Finger has moved while touching. Disabling right-click.");
                        self.state.has_moved = true;
                    } else {
                        // check if we pressed long enough to trigger a right-click
                        let touch_start_time = self.state.touch_start_time.unwrap();
                        let time_touching = Instant::now().duration_since(touch_start_time);

                        if time_touching > RIGHT_CLICK_THRESHOLD {
                            log::info!("Starting right-click.");
                            self.state.is_right_click = true;
                            events.emit_btn_press(BTN_RIGHT);
                        }
                    }
                }
            }
        }
        self.state.touch_state = packet.touch_state();

        self.state.set_x(packet.x());
        events.emit_move_x(packet.x(), &self.monitor_cfg);

        self.state.set_y(packet.y());
        events.emit_move_y(packet.y(), &self.monitor_cfg);

        events.finish()
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
        u.enable_property(&InputProp::INPUT_PROP_DIRECT)?;

        u.enable_event_type(&EventType::EV_KEY)?;
        u.enable_event_code(&EventCode::EV_KEY(BTN_LEFT), None)?;
        u.enable_event_code(&EventCode::EV_KEY(BTN_RIGHT), None)?;

        let abs_info_x: AbsInfo = AbsInfo {
            value: 0,
            minimum: self.monitor_cfg.screen_space.x().min.value(),
            maximum: self.monitor_cfg.screen_space.x().max.value(),
            // TODO test if fuzz value works as expected. should remove spurious drags when pressing long for right-click
            fuzz: 50,
            flat: 0,
            resolution: 0,
        };

        let abs_info_y: AbsInfo = AbsInfo {
            value: 0,
            minimum: self.monitor_cfg.screen_space.y().min.value(),
            maximum: self.monitor_cfg.screen_space.y().max.value(),
            fuzz: 50,
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

    fn send_events(&self, vm: &UInputDevice, events: &[InputEvent]) -> Result<(), EgalaxError> {
        for event in events {
            vm.write_event(event)?;
        }

        Ok(())
    }
}

struct EventGen {
    time: TimeVal,
    events: Vec<InputEvent>,
}

impl EventGen {
    fn new(time: TimeVal) -> Self {
        Self {
            time,
            events: Vec::new(),
        }
    }

    fn emit_btn_press(&mut self, btn: EV_KEY) {
        self.events
            .push(InputEvent::new(&self.time, &EventCode::EV_KEY(btn), 1));
    }

    fn emit_btn_release(&mut self, btn: EV_KEY) {
        self.events
            .push(InputEvent::new(&self.time, &EventCode::EV_KEY(btn), 0));
    }

    fn emit_move_x(&mut self, x: dimX, monitor_cfg: &MonitorConfig) {
        let t = monitor_cfg.calibration_points.x().linear_factor(x);
        let xm = monitor_cfg.monitor_area.x().lerp(t);

        log::info!("Moving to x {}", xm.value());
        self.events.push(InputEvent::new(
            &self.time,
            &EventCode::EV_ABS(EV_ABS::ABS_X),
            xm.value(),
        ));
    }

    fn emit_move_y(&mut self, y: dimY, monitor_cfg: &MonitorConfig) {
        let t = monitor_cfg.calibration_points.y().linear_factor(y);
        let ym = monitor_cfg.monitor_area.y().lerp(t);

        log::info!("Moving to y {}", ym.value());
        self.events.push(InputEvent::new(
            &self.time,
            &EventCode::EV_ABS(EV_ABS::ABS_Y),
            ym.value(),
        ));
    }

    fn emit_syn(&mut self) {
        self.events.push(InputEvent::new(
            &self.time,
            &EventCode::EV_SYN(EV_SYN::SYN_REPORT),
            0,
        ))
    }
    fn finish(mut self) -> Vec<InputEvent> {
        self.emit_syn();
        self.events
    }
}

#[derive(Debug, PartialEq)]
struct DriverState {
    touch_state: TouchState,
    is_right_click: bool,
    has_moved: bool,
    p: Point,
    touch_start_time: Option<Instant>,
    touch_origin: Option<Point>,
}

impl DriverState {
    #[allow(dead_code)]
    pub fn touch_state(&self) -> TouchState {
        self.touch_state
    }

    #[allow(dead_code)]
    pub fn x(&self) -> dimX {
        self.p.x
    }

    pub fn set_x(&mut self, x: dimX) -> () {
        self.p.x = x;
    }

    #[allow(dead_code)]
    pub fn y(&self) -> dimY {
        self.p.y
    }

    pub fn set_y(&mut self, y: dimY) -> () {
        self.p.y = y;
    }

    pub fn reset(&mut self) {
        self.is_right_click = false;
        self.has_moved = false;
        self.touch_start_time = None;
        self.touch_origin = None;
    }
}

impl Default for DriverState {
    fn default() -> Self {
        DriverState {
            touch_state: TouchState::NotTouching,
            is_right_click: false,
            has_moved: false,
            p: (0, 0).into(),
            touch_start_time: None,
            touch_origin: None,
        }
    }
}

#[derive(Debug)]
pub enum EgalaxError {
    UnexpectedEOF,
    DeviceError,
    MonitorNotFound(String),
    TimeError(time::SystemTimeError),
    ParseError(ParsePacketError),
    IOError(io::Error),
    Xrandr(xrandr::XrandrError),
    SerdeLexpr(serde_lexpr::Error),
}

impl From<time::SystemTimeError> for EgalaxError {
    fn from(e: time::SystemTimeError) -> Self {
        Self::TimeError(e)
    }
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

impl From<xrandr::XrandrError> for EgalaxError {
    fn from(e: xrandr::XrandrError) -> Self {
        Self::Xrandr(e)
    }
}

impl From<serde_lexpr::Error> for EgalaxError {
    fn from(e: serde_lexpr::Error) -> Self {
        Self::SerdeLexpr(e)
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
            EgalaxError::Xrandr(e) => return e.fmt(f),
            EgalaxError::SerdeLexpr(e) => return e.fmt(f),
            EgalaxError::UnexpectedEOF => String::from("Unexpected EOF"),
            EgalaxError::DeviceError => String::from("Device Error"),
            EgalaxError::MonitorNotFound(name) => format!("Monitor \"{}\" not found", name),
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
        stream.read_exact(&mut raw_packet)?;
        // log::info!("Successfully read raw packet.");
        let time = TimeVal::try_from(SystemTime::now())?;
        let packet = Packet::try_from(raw_packet)?;
        f(packet.with_time(time))?;
    }
}

/// Print the sequence of packets in the given stream
pub fn print_packets(stream: &mut impl io::Read) -> Result<(), EgalaxError> {
    process_packets(stream, &mut |packet| Ok(println!("{:#?}", packet)))
}

/// Send evdev events for a virtual mouse based on the packets in the given stream
pub fn virtual_mouse(
    mut stream: impl io::Read,
    monitor_cfg: MonitorConfig,
) -> Result<(), EgalaxError> {
    let mut driver = Driver::new(monitor_cfg);
    let vm = driver.get_virtual_device()?;
    log::info!(
        "Successfully set up virtual input device with device node {}",
        vm.devnode().unwrap_or("<unknown>")
    );

    let mut process_packet = |packet| {
        let events = driver.update(packet);
        driver.send_events(&vm, &events)
    };
    process_packets(&mut stream, &mut process_packet)
}
