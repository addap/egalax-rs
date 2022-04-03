use std::path::Path;

fn main() {
    let context = libudev::Context::new().unwrap();
    let mut enumerator = libudev::Enumerator::new(&context).unwrap();

    enumerator.match_subsystem("usb").unwrap();

    // for device in enumerator.scan_devices().unwrap() {
    //     println!(
    //         "found device at node {:?} with driver {:?}",
    //         device.syspath(),
    //         device.driver()
    //     );
    // }

    let device = libudev::Device::from_syspath(
        &context,
        Path::new("/sys/devices/pci0000:00/0000:00:08.1/0000:07:00.3/usb3/3-2/3-2:1.0"),
    )
    .unwrap();

    for property in device.properties() {
        println!("{:?} = {:?}", property.name(), property.value());
    }
    for attribute in device.attributes() {
        println!("{:?} = {:?}", attribute.name(), attribute.value());
    }
}
