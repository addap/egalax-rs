use evdev_rs::enums::{BusType, EventCode, EventType, InputProp, EV_ABS, EV_KEY, EV_SYN};
use evdev_rs::{
    AbsInfo, DeviceWrapper, EnableCodeData, InputEvent, TimeVal, UInputDevice, UninitDevice,
};
use std::time::{Duration, Instant, SystemTime};
use std::{io, thread};

use crate::config::Config;
use crate::error::EgalaxError;
use crate::geo::Point2D;
use crate::protocol::{PacketTag, RawPacket, TouchState, USBMessage, USBPacket, RAW_PACKET_LEN};

/// Touchstate of the driver that also keeps track of when & where the touch started.
#[derive(Debug, Clone, Copy)]
enum DriverTouchState {
    IsTouching {
        /// The start time of the current touch.
        touch_start_time: Instant,
        /// The initial touch point.
        touch_origin: Point2D,
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
}

impl DriverState {
    pub fn touch_state(&self) -> DriverTouchState {
        self.touch_state
    }
}

impl Default for DriverState {
    fn default() -> Self {
        DriverState {
            touch_state: DriverTouchState::NotTouching,
            is_right_click: false,
            has_moved: false,
        }
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

    fn add_btn_press(&mut self, btn: EV_KEY) {
        self.events
            .push(InputEvent::new(&self.time, &EventCode::EV_KEY(btn), 1));
    }

    fn add_btn_release(&mut self, btn: EV_KEY) {
        self.events
            .push(InputEvent::new(&self.time, &EventCode::EV_KEY(btn), 0));
    }

    fn add_move_position(&mut self, position: Point2D, monitor_cfg: &Config) {
        let x_scale = monitor_cfg
            .calibration_points()
            .x()
            .linear_factor(position.x);
        let x_monitor = monitor_cfg.monitor_area.x().lerp(x_scale);

        let y_scale = monitor_cfg
            .calibration_points()
            .y()
            .linear_factor(position.y);
        let y_monitor = monitor_cfg.monitor_area.y().lerp(y_scale);

        log::info!("Moving to x {}", x_monitor.value());
        log::info!("Moving to y {}", y_monitor.value());

        self.events.push(InputEvent::new(
            &self.time,
            &EventCode::EV_ABS(EV_ABS::ABS_X),
            x_monitor.int(),
        ));
        self.events.push(InputEvent::new(
            &self.time,
            &EventCode::EV_ABS(EV_ABS::ABS_Y),
            y_monitor.int(),
        ));
    }

    fn add_syn(&mut self) {
        self.events.push(InputEvent::new(
            &self.time,
            &EventCode::EV_SYN(EV_SYN::SYN_REPORT),
            0,
        ))
    }

    fn finish(mut self) -> Vec<InputEvent> {
        self.add_syn();
        self.events
    }
}

/// Driver contains its current state and config used for processing touchscreen packets.
#[derive(Debug)]
struct Driver {
    state: DriverState,
    config: Config,
}

impl Driver {
    /// Create a new driver with default initial state from a config.
    fn new(monitor_cfg: Config) -> Self {
        Self {
            state: DriverState::default(),
            config: monitor_cfg,
        }
    }

    /// Update the internal state of the driver and return any evdev events that should be emitted.
    /// Linux' input subsystem already filters out duplicate events so we always emit moves to x & y.
    fn update(&mut self, message: USBMessage) -> Vec<InputEvent> {
        log::trace!("Entering Driver::update");

        log::info!("Processing message: {}", message);

        let mut events = EventGen::new(message.time());
        let packet = message.packet();

        match (self.state.touch_state(), packet.touch_state()) {
            (DriverTouchState::NotTouching, TouchState::NotTouching) => {
                // No touch previously and now.
            }
            (DriverTouchState::IsTouching { .. }, TouchState::NotTouching) => {
                // User stopped touching so we release any buttons and reset the state.
                // TODO explain why we release both left and right click. Another approach would be to wait until this point to issue both a left click and release.
                log::info!("Releasing left-click.");
                events.add_btn_release(self.config.ev_left_click());

                if self.state.is_right_click {
                    log::info!("Releasing right-click.");
                    events.add_btn_release(self.config.ev_right_click());
                }

                self.state = DriverState::default();
            }
            (DriverTouchState::NotTouching, TouchState::IsTouching) => {
                // User started touching so we start with a left-click.
                log::info!("Starting left-click.");
                self.state.touch_state = DriverTouchState::IsTouching {
                    touch_start_time: Instant::now(),
                    touch_origin: packet.position(),
                };
                events.add_btn_press(self.config.ev_left_click());
            }
            (
                DriverTouchState::IsTouching {
                    touch_start_time,
                    touch_origin,
                },
                TouchState::IsTouching,
            ) => {
                // User continues touching.
                // During a continued touch we check whether the finger moved too far and if so we disable right-clicks.
                // And otherwise we perform a right-click if the user pressed long enough.
                if !self.state.is_right_click && !self.state.has_moved {
                    let touch_distance = touch_origin.euclidean_distance_to(&packet.position());

                    if touch_distance > self.config.has_moved_threshold() {
                        log::info!("Finger has moved while touching. Disabling right-click.");
                        self.state.has_moved = true;
                    } else {
                        let time_touching = Instant::now().duration_since(touch_start_time);

                        if time_touching > self.config.right_click_wait() {
                            log::info!("Starting right-click.");
                            self.state.is_right_click = true;
                            events.add_btn_press(self.config.ev_right_click());
                        }
                    }
                }
            }
        }

        events.add_move_position(packet.position(), &self.config);
        events.finish()
    }

    /// Setup the virtual device with uinput
    /// Customized from https://github.com/ndesh26/evdev-rs/blob/master/examples/vmouse.rs
    fn get_virtual_device(&self) -> Result<UInputDevice, EgalaxError> {
        log::trace!("Entering Driver::get_virtual_device.");

        let u = UninitDevice::new().ok_or(EgalaxError::Device)?;

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
        u.enable_event_code(&EventCode::EV_KEY(self.config.ev_left_click()), None)?;
        u.enable_event_code(&EventCode::EV_KEY(self.config.ev_right_click()), None)?;

        // For the minimum and maximum values we must specify the whole virtual screen space
        // to establish a frame of reference. Later, we will always send cursor movements
        // that are restricted to the screen space of the designated monitor.
        let abs_info_x: AbsInfo = AbsInfo {
            value: 0,
            minimum: self.config.screen_space.x().min.int(),
            maximum: self.config.screen_space.x().max.int(),
            // TODO test if fuzz value works as expected. should remove spurious drags when pressing long for right-click
            fuzz: 50,
            flat: 0,
            resolution: 0,
        };

        let abs_info_y: AbsInfo = AbsInfo {
            value: 0,
            minimum: self.config.screen_space.y().min.int(),
            maximum: self.config.screen_space.y().max.int(),
            fuzz: 50,
            flat: 0,
            resolution: 0,
        };

        u.enable_event_type(&EventType::EV_ABS)?;
        u.enable_event_code(
            &EventCode::EV_ABS(EV_ABS::ABS_X),
            Some(EnableCodeData::AbsInfo(abs_info_x)),
        )?;
        u.enable_event_code(
            &EventCode::EV_ABS(EV_ABS::ABS_Y),
            Some(EnableCodeData::AbsInfo(abs_info_y)),
        )?;

        // TODO do we need MSC_SCAN which is present in recording.txt?
        u.enable_event_code(&EventCode::EV_SYN(EV_SYN::SYN_REPORT), None)?;

        // Attempt to create UInputDevice from UninitDevice
        log::info!("Create virtual device using uinput.");
        let vm = UInputDevice::create_from_device(&u).map_err(EgalaxError::IO)?;

        // We are supposed to sleep for a small amount of time so that udev can register the device
        thread::sleep(Duration::from_secs(1));

        log::trace!("Leaving Driver::get_virtual_device.");
        Ok(vm)
    }

    /// Send the generated events to the uinput virtual device.
    fn send_events(&self, vm: &UInputDevice, events: &[InputEvent]) -> Result<(), EgalaxError> {
        log::trace!("Entering Driver::send_events.");

        for event in events {
            vm.write_event(event)?;
        }

        log::trace!("Leaving Driver::send_events.");
        Ok(())
    }
}

/// Call a function on all packets in the given stream
pub fn process_packets<T, F>(stream: &mut T, mut f: F) -> Result<(), EgalaxError>
where
    T: io::Read,
    F: FnMut(USBMessage) -> Result<(), EgalaxError>,
{
    let mut raw_packet = RawPacket([0; RAW_PACKET_LEN]);

    loop {
        match stream.read_exact(&mut raw_packet.0) {
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(()),
            res => res?,
        };
        log::info!("Read raw packet: {}", raw_packet);

        let time = TimeVal::try_from(SystemTime::now())?;
        let packet = USBPacket::try_parse(raw_packet, Some(PacketTag::TouchEvent))?;
        f(packet.with_time(time))?;
    }
}

/// Create a virtual mouse using uinput and then continuously transform packets from the touchscreen into
/// evdev events that move the mouse.
pub fn virtual_mouse<T>(stream: &mut T, monitor_cfg: Config) -> Result<(), EgalaxError>
where
    T: io::Read,
{
    log::trace!("Entering fn virtual_mouse");

    let mut driver = Driver::new(monitor_cfg);
    let vm = driver.get_virtual_device()?;

    log::info!(
        "Successfully set up virtual input device with device node {}",
        vm.devnode().unwrap_or("<unknown>")
    );

    let process_packet = |message| {
        let events = driver.update(message);
        driver.send_events(&vm, &events)
    };
    process_packets(stream, process_packet)?;

    log::trace!("Leaving fn virtual_mouse");
    Ok(())
}
