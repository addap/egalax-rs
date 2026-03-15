use anyhow::Context;
use evdev_rs::enums::{BusType, EventCode, EventType, InputProp, EV_ABS, EV_KEY, EV_SYN};
use evdev_rs::{
    AbsInfo, DeviceWrapper, EnableCodeData, InputEvent, TimeVal, UInputDevice, UninitDevice,
};
use std::time::{Duration, Instant, SystemTime};
use std::{io, thread};

use crate::config::Config;
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
}

impl DriverState {
    pub fn touch_state(&self) -> DriverTouchState {
        self.touch_state
    }
}

impl Default for DriverState {
    fn default() -> Self {
        Self {
            touch_state: DriverTouchState::NotTouching,
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

    fn add_btn_click(&mut self, btn: EV_KEY) {
        self.add_btn_press(btn);
        self.add_syn();
        self.add_btn_release(btn);
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
        let position = monitor_cfg.calibration_points().clamp(position);

        log::info!("Moving to xy={}", position);

        self.events.push(InputEvent::new(
            &self.time,
            &EventCode::EV_ABS(EV_ABS::ABS_X),
            position.x.value(),
        ));
        self.events.push(InputEvent::new(
            &self.time,
            &EventCode::EV_ABS(EV_ABS::ABS_Y),
            position.y.value(),
        ));
    }

    fn add_syn(&mut self) {
        self.events.push(InputEvent::new(
            &self.time,
            &EventCode::EV_SYN(EV_SYN::SYN_REPORT),
            0,
        ));
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
            (DriverTouchState::IsTouching { .. }, TouchState::IsTouching) => {
                // Was touching and still touching.
                // Just add the coordinates (happens below) and do nothing else.
            }
            (DriverTouchState::IsTouching { .. }, TouchState::NotTouching) => {
                // User stopped touching.
                log::info!("Releasing touch.");

                events.add_btn_release(EV_KEY::BTN_TOUCH);

                self.state = DriverState::default();
            }
            (DriverTouchState::NotTouching, TouchState::IsTouching) => {
                // User started touching.
                log::info!("Starting touch.");
                self.state.touch_state = DriverTouchState::IsTouching {
                    touch_start_time: Instant::now(),
                    touch_origin: packet.position(),
                };
                events.add_btn_press(EV_KEY::BTN_TOUCH);
            }
        }

        events.add_move_position(packet.position(), &self.config);
        events.finish()
    }

    /// Setup the virtual device with uinput.
    /// Customized from <https://github.com/ndesh26/evdev-rs/blob/master/examples/vmouse.rs>
    fn get_virtual_device(&self) -> anyhow::Result<UInputDevice> {
        log::trace!("Entering Driver::get_virtual_device.");

        let u = UninitDevice::new().context("Unable to set up libevdev device struct.")?;

        // Setup device
        // as per: https://01.org/linuxgraphics/gfx-docs/drm/input/uinput.html#mouse-movements

        log::info!("Set basic properties of virtual device.");
        u.set_name("Egalax Virtual Mouse");
        u.set_bustype(BusType::BUS_USB as u16);
        u.set_vendor_id(0x0eef);
        u.set_product_id(0xcafe);
        u.enable_property(&InputProp::INPUT_PROP_DIRECT)?;

        u.enable_event_type(&EventType::EV_KEY)?;
        u.enable_event_code(&EventCode::EV_KEY(EV_KEY::BTN_TOUCH), None)?;

        // For the minimum and maximum values we must specify the whole virtual screen space
        // to establish a frame of reference. Later, we will always send cursor movements
        // that are restricted to the screen space of the designated monitor.
        let abs_info_x: AbsInfo = AbsInfo {
            value: 0,
            minimum: self.config.calibration_points().xrange().min().value(),
            maximum: self.config.calibration_points().xrange().max().value(),
            // TODO test if fuzz value works as expected.
            // A.R: maybe make it configurable
            fuzz: 50,
            flat: 0,
            resolution: 0,
        };

        let abs_info_y: AbsInfo = AbsInfo {
            value: 0,
            minimum: self.config.calibration_points().yrange().min().value(),
            maximum: self.config.calibration_points().yrange().max().value(),
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
        u.enable_event_type(&EventType::EV_SYN)?;
        u.enable_event_code(&EventCode::EV_SYN(EV_SYN::SYN_REPORT), None)?;

        // Attempt to create UInputDevice from UninitDevice
        log::info!("Create virtual device using uinput.");
        let vm = UInputDevice::create_from_device(&u)
            .context("Unable to set up uninput device. Check permissions for /dev/uninput.")?;

        // We are supposed to sleep for a small amount of time so that udev can register the device
        thread::sleep(Duration::from_secs(1));

        log::trace!("Leaving Driver::get_virtual_device.");
        Ok(vm)
    }
}

/// Send the generated events to the uinput virtual device.
fn send_events(vm: &UInputDevice, events: &[InputEvent]) -> anyhow::Result<()> {
    log::trace!("Entering driver::send_events.");

    for event in events {
        vm.write_event(event)?;
    }

    log::trace!("Leaving driver::send_events.");
    Ok(())
}

/// Call a function on all packets in the given stream
pub fn process_packets<T, F>(stream: &mut T, mut f: F) -> anyhow::Result<()>
where
    T: io::Read,
    F: FnMut(USBMessage) -> anyhow::Result<()>,
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
pub fn virtual_mouse<T>(stream: &mut T, monitor_cfg: Config) -> anyhow::Result<()>
where
    T: io::Read,
{
    log::trace!("Entering fn virtual_mouse");

    let mut driver = Driver::new(monitor_cfg);
    let vm = driver
        .get_virtual_device()
        .context("Unable to set up virtual device.")?;

    log::info!(
        "Successfully set up virtual input device with device node {}",
        vm.devnode().unwrap_or("<unknown>")
    );

    let process_packet = |message| {
        let events = driver.update(message);
        send_events(&vm, &events)
    };
    process_packets(stream, process_packet)?;

    log::trace!("Leaving fn virtual_mouse");
    Ok(())
}
