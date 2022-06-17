use egalax_rs::protocol::{Packet, RawPacket, TouchState};
use std::error;
use std::result::Result;

fn main() -> Result<(), Box<dyn error::Error>> {
    let packet = Packet::try_parse(RawPacket([0x02, 0x03, 0x3b, 0x01, 0x32, 0x01]), None)?;
    println!(
        "Finger is {} at ({}, {}) with resolution {}",
        match packet.touch_state() {
            TouchState::IsTouching => "touching",
            TouchState::NotTouching => "not touching",
        },
        packet.x(),
        packet.y(),
        packet.res()
    );
    println!("{:#?}", packet);

    Ok(())
}
