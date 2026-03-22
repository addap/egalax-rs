//! Print out the reports captured in hidraw.bin.

use egalax_rs::driver::process_reports;
use std::{error, fs, io::Cursor};

const HIDRAW_FILE: &str = "./dumps/hidraw.bin";

fn main() -> Result<(), Box<dyn error::Error>> {
    env_logger::init();
    let hidraw = fs::read(HIDRAW_FILE).expect("Cannot read hidraw file");
    let mut stream = Cursor::new(hidraw);

    let process_report = |report| {
        println!("{}", report);
        Ok(())
    };
    process_reports(&mut stream, process_report)?;
    Ok(())
}
