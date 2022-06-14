use egalax_rs::config::MonitorConfigBuilder;
use egalax_rs::driver::virtual_mouse;
use std::result::Result;
use std::{error, fs::OpenOptions};

fn main() -> Result<(), Box<dyn error::Error>> {
    env_logger::init();

    let usage = "usage: sudo ./target/debug/egalax-rs /dev/hidraw0";

    let node_path = std::env::args().nth(1).expect(usage);
    log::info!("Using raw device node '{}'", node_path);
    let device_node = OpenOptions::new().read(true).open(&node_path).unwrap();

    let monitor_cfg =
        MonitorConfigBuilder::from_file("/home/adrian/programming/rust/egalax-rs/config")?
            .build()?;
    log::info!("Using monitor config {}", monitor_cfg);

    virtual_mouse(device_node, monitor_cfg)?;
    Ok(())
}
