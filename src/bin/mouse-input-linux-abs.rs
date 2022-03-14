use std::os::unix::fs::OpenOptionsExt;
use std::{fs::OpenOptions, io, thread, time::Duration};

use input_linux::{
    AbsoluteAxis, AbsoluteEvent, AbsoluteInfoSetup, EventKind, EventTime, InputEvent, InputId, Key,
    KeyEvent, KeyState, RelativeAxis, RelativeEvent, SynchronizeEvent, SynchronizeKind,
    UInputHandle,
};
use nix::libc::O_NONBLOCK;

// A customization of mouse-input-linux.rs to use a device with absolute axes.
fn main() -> io::Result<()> {
    let uinput_file = OpenOptions::new()
        .read(true)
        .write(true)
        .custom_flags(O_NONBLOCK)
        .open("/dev/uinput")?;
    let uhandle = UInputHandle::new(uinput_file);

    uhandle.set_evbit(EventKind::Key)?;
    uhandle.set_keybit(Key::ButtonTouch)?;

    uhandle.set_evbit(EventKind::Absolute)?;
    uhandle.set_absbit(AbsoluteAxis::X)?;
    uhandle.set_absbit(AbsoluteAxis::Y)?;

    let input_id = InputId {
        bustype: input_linux::sys::BUS_USB,
        vendor: 0x1234,
        product: 0x5678,
        version: 0,
    };
    let device_name = b"Egalax Example Device";
    uhandle.create(
        &input_id,
        device_name,
        0,
        &[
            AbsoluteInfoSetup {
                axis: AbsoluteAxis::X,
                info: input_linux::AbsoluteInfo {
                    value: 0,
                    minimum: 30,
                    maximum: 4040,
                    fuzz: 0,
                    flat: 0,
                    resolution: 0,
                },
            },
            AbsoluteInfoSetup {
                axis: AbsoluteAxis::Y,
                info: input_linux::AbsoluteInfo {
                    value: 0,
                    minimum: 60,
                    maximum: 4035,
                    fuzz: 0,
                    flat: 0,
                    resolution: 0,
                },
            },
        ],
    )?;

    // This call to sleep was not necessary on my machine,
    // but this translation is meant to match exactly
    thread::sleep(Duration::from_secs(1));

    for i in 0..10 {
        const ZERO: EventTime = EventTime::new(0, 0);
        let events = [
            *InputEvent::from(KeyEvent::new(
                ZERO,
                Key::ButtonTouch,
                KeyState::pressed(true),
            ))
            .as_raw(),
            *InputEvent::from(AbsoluteEvent::new(ZERO, AbsoluteAxis::X, 100 * i)).as_raw(),
            *InputEvent::from(AbsoluteEvent::new(ZERO, AbsoluteAxis::Y, 100 * i)).as_raw(),
            *InputEvent::from(SynchronizeEvent::new(ZERO, SynchronizeKind::Report, 0)).as_raw(),
        ];
        uhandle.write(&events)?;
        thread::sleep(Duration::from_secs(1));

        let events2 = [*InputEvent::from(KeyEvent::new(
            ZERO,
            Key::ButtonTouch,
            KeyState::pressed(false),
        ))
        .as_raw()];
        uhandle.write(&events2)?;

        thread::sleep(Duration::from_secs(1));
    }

    // This call to sleep was not necessary on my machine,
    // but this translation is meant to match exactly
    thread::sleep(Duration::from_secs(1));
    uhandle.dev_destroy()?;

    Ok(())
}
