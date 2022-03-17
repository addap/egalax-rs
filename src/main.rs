use egalax_rs::config::MonitorConfigBuilder;
use egalax_rs::driver::virtual_mouse;
use std::result::Result;
use std::{error, fs::OpenOptions};

fn main() -> Result<(), Box<dyn error::Error>> {
    env_logger::init();

    let usage = "usage: sudo ./target/debug/egalax-rs /dev/hidraw0 [HDMI-A-0]";

    let node_path = std::env::args().nth(1).expect(usage);
    log::info!("Using raw device node {}", node_path);
    let device_node = OpenOptions::new().read(true).open(&node_path).unwrap();

    let monitor_name = std::env::args().nth(2);
    // a.d. have to match on .as_ref() so that we don't consume the option
    if let Some(monitor_name) = monitor_name.as_ref() {
        log::info!("Using xrandr monitor name {}", monitor_name);
    }
    let monitor_cfg = MonitorConfigBuilder::new()?
        .with_name(monitor_name)
        .build()?;
    log::info!("Using monitor config {}", monitor_cfg);

    virtual_mouse(device_node, monitor_cfg)?;
    Ok(())
}
