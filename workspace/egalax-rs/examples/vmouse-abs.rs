use evdev_rs::enums::{BusType, EventCode, EventType, EV_ABS, EV_KEY, EV_SYN};
use evdev_rs::{
    AbsInfo, DeviceWrapper, EnableCodeData, InputEvent, TimeVal, UInputDevice, UninitDevice,
};
use std::thread::{self, sleep};
use std::time::Duration;

const SIZE: i32 = 1 << 12;

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
    u.enable_event_code(&EventCode::EV_KEY(EV_KEY::BTN_LEFT), None)?;

    let abs_info_x: AbsInfo = AbsInfo {
        value: 0,
        minimum: 0,
        maximum: SIZE,
        fuzz: 0,
        flat: 0,
        resolution: 0,
    };

    let abs_info_y: AbsInfo = AbsInfo {
        value: 0,
        minimum: 0,
        maximum: SIZE,
        fuzz: 0,
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

    u.enable_event_code(&EventCode::EV_SYN(EV_SYN::SYN_REPORT), None)?;

    // Attempt to create UInputDevice from UninitDevice
    let v = UInputDevice::create_from_device(&u)?;
    sleep(Duration::from_secs(1));
    Ok(v)
}

fn main() -> Result<(), std::io::Error> {
    let v = mkdev()?;
    let zero = TimeVal::new(0, 0);

    // v.write_event(&InputEvent {
    //     time: zero,
    //     event_code: EventCode::EV_KEY(EV_KEY::BTN_LEFT),
    //     value: 1,
    // })?;

    v.write_event(&InputEvent {
        time: zero,
        event_code: EventCode::EV_SYN(EV_SYN::SYN_REPORT),
        value: 0,
    })?;

    for i in 0..=10 {
        // Write mapped event
        v.write_event(&InputEvent {
            time: zero,
            event_code: EventCode::EV_ABS(EV_ABS::ABS_X),
            value: (SIZE / 10) * i,
        })?;

        v.write_event(&InputEvent {
            time: zero,
            event_code: EventCode::EV_ABS(EV_ABS::ABS_Y),
            value: (SIZE / 10) * i,
        })?;

        v.write_event(&InputEvent {
            time: zero,
            event_code: EventCode::EV_SYN(EV_SYN::SYN_REPORT),
            value: 0,
        })?;

        thread::sleep(Duration::from_millis(500));
    }

    // v.write_event(&InputEvent {
    //     time: zero,
    //     event_code: EventCode::EV_KEY(EV_KEY::BTN_TOUCH),
    //     value: 0,
    // })?;

    v.write_event(&InputEvent {
        time: zero,
        event_code: EventCode::EV_SYN(EV_SYN::SYN_REPORT),
        value: 0,
    })?;
    Ok(())
}
