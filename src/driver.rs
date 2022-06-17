use crate::config::MonitorConfig;
use crate::error::EgalaxError;
use crate::protocol::MessageType;
use crate::protocol::{Packet, RawPacket, TouchState, RAW_PACKET_LEN};
use crate::{geo::Point, units::*};
use evdev_rs::enums::{BusType, EventCode, EventType, InputProp, EV_ABS, EV_KEY, EV_SYN};
use evdev_rs::{AbsInfo, DeviceWrapper, InputEvent, TimeVal, UInputDevice, UninitDevice};
use std::time::{Duration, Instant, SystemTime};
use std::{io, thread};

// TODO test values for has_moved thresh
const HAS_MOVED_THRESHOLD: f64 = 30.0;
const RIGHT_CLICK_THRESHOLD: Duration = Duration::from_millis(1500);
const BTN_LEFT: EV_KEY = EV_KEY::BTN_TOUCH;
const BTN_RIGHT: EV_KEY = EV_KEY::BTN_STYLUS2;

/// Driver contains its current state and config used for processing touchscreen packets.
#[derive(Debug)]
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
    /// Linux' input subsystem already filters out duplicate events so we always emit moves to x & y.
    fn update(&mut self, packet: Packet) -> Vec<InputEvent> {
        log::debug!("Entering Driver::update");

        log::info!("Processing packet: {}", packet);

        let mut events = EventGen::new(packet.time());

        // Compare last with current touch state
        match (self.state.touch_state(), packet.touch_state()) {
            (DriverTouchState::NotTouching, TouchState::NotTouching) => {}
            // User stopped touching.
            (DriverTouchState::IsTouching { .. }, TouchState::NotTouching) => {
                log::info!("Releasing left-click.");
                events.emit_btn_release(BTN_LEFT);

                if self.state.is_right_click {
                    log::info!("Releasing right-click.");
                    events.emit_btn_release(BTN_RIGHT);
                }

                // Reset state.
                self.state.touch_state = DriverTouchState::NotTouching;
                self.state.is_right_click = false;
                self.state.has_moved = false;
            }
            // User started touching.
            (DriverTouchState::NotTouching, TouchState::IsTouching) => {
                log::info!("Starting left-click.");
                let touch_start_time = Instant::now();
                let touch_origin = Point::from((packet.x(), packet.y()));
                self.state.touch_state = DriverTouchState::IsTouching {
                    touch_start_time,
                    touch_origin,
                };
                events.emit_btn_press(BTN_LEFT);
            }
            // User continues touching.
            (
                DriverTouchState::IsTouching {
                    touch_start_time,
                    touch_origin,
                },
                TouchState::IsTouching,
            ) => {
                if !self.state.is_right_click && !self.state.has_moved {
                    // check if during press we moved too far away from origin and disable right-click
                    let touch_distance =
                        touch_origin.euc_distance_to(&Point::from((packet.x(), packet.y())));

                    if touch_distance > HAS_MOVED_THRESHOLD {
                        log::info!("Finger has moved while touching. Disabling right-click.");
                        self.state.has_moved = true;
                    } else {
                        // check if we pressed long enough to trigger a right-click
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

        self.state.set_x(packet.x());
        events.emit_move_x(packet.x(), &self.monitor_cfg);

        self.state.set_y(packet.y());
        events.emit_move_y(packet.y(), &self.monitor_cfg);

        events.finish()
    }

    /// Setup the virtual device with uinput
    /// Customized from https://github.com/ndesh26/evdev-rs/blob/master/examples/vmouse.rs
    fn get_virtual_device(&self) -> Result<UInputDevice, EgalaxError> {
        log::debug!("Entering Driver::get_virtual_device.");

        log::info!("Start setting up virtual device.");
        let u = UninitDevice::new().ok_or(EgalaxError::DeviceError)?;

        // Setup device
        // per: https://01.org/linuxgraphics/gfx-docs/drm/input/uinput.html#mouse-movements

        log::info!("Set basic properties of virtual device.");
        u.set_name("Egalax Virtual Mouse");
        u.set_bustype(BusType::BUS_USB as u16);
        u.set_vendor_id(0x0eef);
        u.set_product_id(0xcafe);
        u.enable_property(&InputProp::INPUT_PROP_DIRECT)?;

        log::info!("Set events that will be generated for virtual device.");
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
        log::info!("Create virtual device using uinput.");
        let vm = UInputDevice::create_from_device(&u).map_err(EgalaxError::IOError)?;

        // Apparently you're supposed to sleep for a small amount of time so that udev can register the device
        thread::sleep(Duration::from_secs(1));

        log::debug!("Leaving Driver::get_virtual_device.");
        Ok(vm)
    }

    /// Send the generated events to the uinput virtual device.
    fn send_events(&self, vm: &UInputDevice, events: &[InputEvent]) -> Result<(), EgalaxError> {
        log::debug!("Entering Driver::send_events.");

        for event in events {
            vm.write_event(event)?;
        }

        log::debug!("Leaving Driver::send_events.");
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

#[derive(Debug, Clone, Copy)]
enum DriverTouchState {
    IsTouching {
        /// The start time of the current touch, if someone is currently touching.
        touch_start_time: Instant,
        /// The initial touch point.
        touch_origin: Point,
    },
    NotTouching,
}

/// Internal state of the driver.
#[derive(Debug)]
struct DriverState {
    /// If someone is pressing on the touchscreen.
    touch_state: DriverTouchState,
    /// If we are emitting a right-click.
    is_right_click: bool,
    /// If true, finger has moved too much so we don't emit a right-click.
    has_moved: bool,
    /// Current touch point.
    p: Point,
}

impl DriverState {
    pub fn touch_state(&self) -> DriverTouchState {
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
}

impl Default for DriverState {
    fn default() -> Self {
        DriverState {
            touch_state: DriverTouchState::NotTouching,
            is_right_click: false,
            has_moved: false,
            p: (0, 0).into(),
        }
    }
}

/// Call a function on all packets in the given stream
pub fn process_packets<T, F>(stream: &mut T, f: &mut F) -> Result<(), EgalaxError>
where
    T: io::Read,
    F: FnMut(Packet) -> Result<(), EgalaxError>,
{
    let mut raw_packet = RawPacket([0; RAW_PACKET_LEN]);

    loop {
        stream.read_exact(&mut raw_packet.0)?;
        log::info!("Read raw packet: {}", raw_packet);

        let time = TimeVal::try_from(SystemTime::now())?;
        let packet = Packet::try_parse(raw_packet, Some(MessageType::TouchEvent))?;
        f(packet.with_time(time))?;
    }
}

/// Print the sequence of packets in the given stream
pub fn print_packets(stream: &mut impl io::Read) -> Result<(), EgalaxError> {
    process_packets(stream, &mut |packet| Ok(println!("{:#?}", packet)))
}

/// Create a virtual mouse using uinput and then continuously transform packets from the touchscreen into
/// evdev events that move the mouse.
pub fn virtual_mouse(
    mut stream: impl io::Read,
    monitor_cfg: MonitorConfig,
) -> Result<(), EgalaxError> {
    log::debug!("Entering fn virtual_mouse");

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
