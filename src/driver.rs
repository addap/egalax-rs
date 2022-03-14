use crate::protocol::{Packet, ParsePacketError, RawPacket, RAW_PACKET_LEN};
use crate::{dimX, dimY, Point};
use evdev_rs::enums::{BusType, EventCode, EventType, EV_ABS, EV_KEY, EV_SYN};
use evdev_rs::{
    AbsInfo, Device, DeviceWrapper, InputEvent, ReadFlag, TimeVal, UInputDevice, UninitDevice,
};
use std::time::{self, Duration, SystemTime};
use std::{error, fmt, io, thread};

#[derive(Debug, PartialEq)]
struct Driver {
    touch_state: TouchState,
    ul_bounds: Point,
    lr_bounds: Point,
    monitor_info: MonitorInfo,
}

impl Driver {
    fn new(ul_bounds: Point, lr_bounds: Point) -> Self {
        Self {
            touch_state: TouchState::default(),
            ul_bounds,
            lr_bounds,
            monitor_info: MonitorInfo::default(),
        }
    }

    // TODO implement debouncing
    fn update(&mut self, packet: Packet) -> Vec<ChangeSet> {
        let mut changes = Vec::new();

        if self.touch_state.is_touching != packet.is_touching() {
            self.touch_state.is_touching = packet.is_touching();
            if packet.is_touching() {
                changes.push(ChangeSet::Pressed);
            } else {
                changes.push(ChangeSet::Released);
            }
        }

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

    // copied from evdev-rs' vmouse.rs
    fn get_virtual_device(&self) -> Result<UInputDevice, EgalaxError> {
        // Create virtual device
        let u = UninitDevice::new().ok_or(EgalaxError::DeviceError)?;

        // Setup device
        // per: https://01.org/linuxgraphics/gfx-docs/drm/input/uinput.html#mouse-movements

        u.set_name("Egalax Virtual Mouse");
        u.set_bustype(BusType::BUS_USB as u16);
        u.set_vendor_id(0x0eef);
        u.set_product_id(0xcafe);

        // Note mouse keys have to be enabled for this to be detected
        // as a usable device, see: https://stackoverflow.com/a/64559658/6074942
        u.enable_event_type(&EventType::EV_KEY)?;
        u.enable_event_code(&EventCode::EV_KEY(EV_KEY::BTN_TOUCH), None)?;

        let abs_info_x: AbsInfo = AbsInfo {
            value: 0,
            minimum: self.ul_bounds.x.value().into(),
            maximum: self.lr_bounds.x.value().into(),
            fuzz: 0,
            flat: 0,
            resolution: 0,
        };

        let abs_info_y: AbsInfo = AbsInfo {
            value: 0,
            minimum: self.ul_bounds.y.value().into(),
            maximum: self.lr_bounds.y.value().into(),
            fuzz: 0,
            flat: 0,
            resolution: 0,
        };

        u.enable_event_type(&EventType::EV_ABS)?;
        u.enable_event_code(&EventCode::EV_ABS(EV_ABS::ABS_X), Some(&abs_info_x))?;
        u.enable_event_code(&EventCode::EV_ABS(EV_ABS::ABS_Y), Some(&abs_info_y))?;

        // TODO do we need MSC_SCAN?
        u.enable_event_code(&EventCode::EV_SYN(EV_SYN::SYN_REPORT), None)?;

        // Attempt to create UInputDevice from UninitDevice
        Ok(UInputDevice::create_from_device(&u)
            .map_err(EgalaxError::IOError)
            .unwrap())
    }

    fn send_event(&self, vm: &UInputDevice, changes: &[ChangeSet]) -> Result<(), EgalaxError> {
        // println!("Sending event {:#?}", changes);
        // let time = SystemTime::now()
        //     .try_into()
        //     .map_err(EgalaxError::TimeError)?;

        let ZERO = TimeVal::new(0, 0);

        for change in changes.iter() {
            let event = change.to_input_event(&self.monitor_info, &ZERO)?;
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
}

impl ChangeSet {
    fn to_input_event(
        &self,
        monitor_info: &MonitorInfo,
        time: &TimeVal,
    ) -> Result<InputEvent, EgalaxError> {
        let (code, value) = match self {
            ChangeSet::Pressed => (EventCode::EV_KEY(EV_KEY::BTN_TOUCH), 1),
            ChangeSet::Released => (EventCode::EV_KEY(EV_KEY::BTN_TOUCH), 0),
            ChangeSet::ChangedX(x) => (EventCode::EV_ABS(EV_ABS::ABS_X), x.value()),
            ChangeSet::ChangedY(y) => (EventCode::EV_ABS(EV_ABS::ABS_Y), y.value()),
        };

        Ok(InputEvent::new(time, &code, value as i32))
    }
}

#[derive(Debug, PartialEq)]
struct TouchState {
    is_touching: bool,
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
            p: (0, 0).into(),
        }
    }
}

#[derive(Debug, PartialEq)]
struct MonitorInfo {
    ul: Point,
    lr: Point,
}

// TODO need to get monitor dimensions from xrandr or config file
impl Default for MonitorInfo {
    fn default() -> Self {
        MonitorInfo {
            ul: (0, 0).into(),
            lr: (1000, 1000).into(),
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
        println!("read next packet");
        let read_bytes = stream.read(&mut raw_packet)?;
        if read_bytes == 0 {
            return Ok(());
        } else if read_bytes < RAW_PACKET_LEN {
            return Err(EgalaxError::UnexpectedEOF);
        }
        let packet = Packet::try_from(raw_packet)?;
        f(packet)?;
        thread::sleep(Duration::from_secs(1));
    }
}

/// Print the sequence of packets in the given stream
pub fn print_packets(stream: &mut impl io::Read) -> Result<(), EgalaxError> {
    process_packets(stream, &mut |packet| Ok(println!("{:#?}", packet)))
}

/// Send evdev events for a virtual mouse based on the packets in the given stream
pub fn virtual_mouse(stream: &mut impl io::Read) -> Result<(), EgalaxError> {
    let ul_bounds = (30, 60).into();
    let lr_bounds = (4040, 4035).into();
    let mut state = Driver::new(ul_bounds, lr_bounds);
    let vm = state.get_virtual_device()?;

    let mut process_packet = |packet| {
        let changes = state.update(packet);
        state.send_event(&vm, &changes)
    };
    process_packets(stream, &mut process_packet)
}
