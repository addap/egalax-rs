use egalax_rs::config::MonitorConfigBuilder;
use egalax_rs::driver::virtual_mouse;
use std::result::Result;
use std::{error, fs::OpenOptions};

fn main() -> Result<(), Box<dyn error::Error>> {
    let usage = "usage: sudo ./target/debug/egalax-rs /dev/hidraw0 [HDMI-A-0]";

    let node_path = std::env::args().nth(1).expect(usage);
    let device_node = OpenOptions::new().read(true).open(&node_path).unwrap();

    let monitor_name = std::env::args().nth(2);
    let monitor_cfg = MonitorConfigBuilder::new()?
        .with_name(monitor_name)
        .build()?;

    virtual_mouse(device_node, monitor_cfg)?;
    Ok(())
}
