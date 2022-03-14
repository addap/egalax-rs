use evdev_rs::enums::{BusType, EventCode, EventType, EV_ABS, EV_KEY, EV_REL, EV_SYN};
use evdev_rs::{
    AbsInfo, Device, DeviceWrapper, InputEvent, ReadFlag, TimeVal, UInputDevice, UninitDevice,
};
use std::thread;
use std::time::Duration;

// A customization of vmouse.rs for a device with absolute axes.
fn mkdev() -> Result<UInputDevice, std::io::Error> {
    // Create virtual device
    let u = UninitDevice::new().unwrap();

    u.set_name("Egalax Virtual Mouse");
    u.set_bustype(BusType::BUS_USB as u16);
    u.set_vendor_id(0xabcd);
    u.set_product_id(0xefef);

    // Note mouse keys have to be enabled for this to be detected
    // as a usable device, see: https://stackoverflow.com/a/64559658/6074942
    u.enable_event_type(&EventType::EV_KEY)?;
    u.enable_event_code(&EventCode::EV_KEY(EV_KEY::BTN_TOUCH), None)?;

    let abs_info_x: AbsInfo = AbsInfo {
        value: 0,
        minimum: 30,
        maximum: 4040,
        fuzz: 0,
        flat: 0,
        resolution: 0,
    };

    let abs_info_y: AbsInfo = AbsInfo {
        value: 0,
        minimum: 60,
        maximum: 4035,
        fuzz: 0,
        flat: 0,
        resolution: 0,
    };

    u.enable_event_type(&EventType::EV_ABS)?;
    u.enable_event_code(&EventCode::EV_ABS(EV_ABS::ABS_X), Some(&abs_info_x))?;
    u.enable_event_code(&EventCode::EV_ABS(EV_ABS::ABS_Y), Some(&abs_info_y))?;

    u.enable_event_code(&EventCode::EV_SYN(EV_SYN::SYN_REPORT), None)?;

    // Attempt to create UInputDevice from UninitDevice
    let v = UInputDevice::create_from_device(&u)?;
    Ok(v)
}

fn main() -> Result<(), std::io::Error> {
    let v = mkdev()?;
    let ZERO = TimeVal::new(0, 0);

    v.write_event(&InputEvent {
        time: ZERO,
        event_code: EventCode::EV_KEY(EV_KEY::BTN_TOUCH),
        value: 1,
    })?;

    v.write_event(&InputEvent {
        time: ZERO,
        event_code: EventCode::EV_SYN(EV_SYN::SYN_REPORT),
        value: 0,
    })?;

    for i in 3..12 {
        // Write mapped event
        v.write_event(&InputEvent {
            time: ZERO,
            event_code: EventCode::EV_ABS(EV_ABS::ABS_X),
            value: 30 * i,
        })?;

        v.write_event(&InputEvent {
            time: ZERO,
            event_code: EventCode::EV_ABS(EV_ABS::ABS_Y),
            value: 60 * i,
        })?;

        v.write_event(&InputEvent {
            time: ZERO,
            event_code: EventCode::EV_SYN(EV_SYN::SYN_REPORT),
            value: 0,
        })?;

        thread::sleep(Duration::from_secs(1));
    }

    v.write_event(&InputEvent {
        time: ZERO,
        event_code: EventCode::EV_KEY(EV_KEY::BTN_TOUCH),
        value: 0,
    })?;

    v.write_event(&InputEvent {
        time: ZERO,
        event_code: EventCode::EV_SYN(EV_SYN::SYN_REPORT),
        value: 0,
    })?;
    Ok(())
}
