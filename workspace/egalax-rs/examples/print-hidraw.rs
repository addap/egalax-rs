//! Print out the packets captured in hidraw.bin.

use egalax_rs::driver::process_packets;
use std::{error, fs, io::Cursor};

const HIDRAW_FILE: &str = "./dumps/hidraw.bin";

fn main() -> Result<(), Box<dyn error::Error>> {
    env_logger::init();
    let hidraw = fs::read(HIDRAW_FILE).expect("Cannot read hidraw file");
    let mut stream = Cursor::new(hidraw);

    let process_packet = |packet| {
        println!("{}", packet);
        Ok(())
    };
    process_packets(&mut stream, process_packet)?;
    Ok(())
}
