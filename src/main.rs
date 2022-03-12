use egalax_rs::protocol::Packet;
use std::error;
use std::result::Result;

fn main() -> Result<(), Box<dyn error::Error>> {
    let packet = Packet::try_from([0x02, 0x03, 0x3b, 0x01, 0x32, 0x01])?;
    println!(
        "Finger is {} at ({}, {}) with resolution {}",
        if packet.is_touching() {
            "touching"
        } else {
            "not touching"
        },
        packet.x(),
        packet.y(),
        packet.res()
    );
    println!("{:#?}", packet);

    Ok(())
}
