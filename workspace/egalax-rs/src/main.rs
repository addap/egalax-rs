use std::fs::OpenOptions;
use std::result::Result;

use egalax_rs::config::ConfigFile;
use egalax_rs::driver::virtual_mouse;
use egalax_rs::error::EgalaxError;

const USAGE: &str = "Usage: egalax-rs /dev/hidraw.egalax";

/// Read configuration and delegate to virtual mouse function.
fn main() -> Result<(), EgalaxError> {
    env_logger::init();

    let node_path = std::env::args().nth(1).expect(USAGE);
    log::info!("Using raw device node '{}'", node_path);

    let mut device_node = OpenOptions::new().read(true).open(&node_path).unwrap();
    log::info!("Opened device node '{}'", node_path);

    let monitor_cfg = ConfigFile::from_file("./config.toml")?.build()?;
    log::info!("Using monitor config:\n{}", monitor_cfg);

    virtual_mouse(&mut device_node, monitor_cfg)?;
    Ok(())
}
