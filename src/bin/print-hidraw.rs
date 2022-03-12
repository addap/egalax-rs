use egalax_rs::driver::print_packets;
use std::{error, fs, io::Cursor};

fn main() -> Result<(), Box<dyn error::Error>> {
    let hidraw = fs::read("./hidraw.bin").expect("Cannot read hidraw file");
    let mut stream = Cursor::new(hidraw);

    print_packets(&mut stream)?;
    Ok(())
}
