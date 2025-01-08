mod app;

use app::App;
use std::{fs::OpenOptions, path::PathBuf};

const USAGE: &str = "Usage: egalax-settings [DEVICE_PATH]\n";
const DEFAULT_DEVICE: &str = "/dev/hidraw.egalax";

fn main() -> eframe::Result {
    env_logger::init();

    let device_path: PathBuf = std::env::args()
        .nth(1)
        .unwrap_or(DEFAULT_DEVICE.to_string())
        .into();
    log::info!("Using raw device node `{:?}`", device_path);

    let device_node = OpenOptions::new()
        .read(true)
        .open(&device_path)
        .expect(&format!(
            "Opening `{device_path:?}` failed. USB cable to monitor disconnected?\n{USAGE}",
        ));
    log::info!("Tested opening device node `{:?}`", device_path);
    drop(device_node);

    let monitors = xrandr::XHandle::open()
        .expect("Failed to open connection to X server.")
        .monitors()
        .expect("Failed to enumerate monitors from X server.")
        .into_iter()
        .map(|m| m.name)
        .collect();

    eframe::run_native(
        "egalax settings editor",
        eframe::NativeOptions::default(),
        Box::new(|cc| Ok(Box::new(App::new(device_path, monitors, cc)))),
    )?;
    Ok(())
}
