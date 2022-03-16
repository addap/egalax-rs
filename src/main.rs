use egalax_rs::driver::virtual_mouse;
use std::result::Result;
use std::{error, fs::OpenOptions};

fn main() -> Result<(), Box<dyn error::Error>> {
    let usage = "usage: sudo ./target/debug/egalax-rs /dev/hidraw0 HDMI-A-0";
    let node_path = std::env::args().nth(1).expect(usage);
    let monitor_name = std::env::args().nth(2).expect(usage);
    let device_node = OpenOptions::new().read(true).open(&node_path).unwrap();

    virtual_mouse(device_node, monitor_name)?;
    Ok(())
}
