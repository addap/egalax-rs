use egalax_rs::geo::AABB;
use xrandr::XHandle;

fn main() {
    let monitors = XHandle::open().unwrap().monitors().unwrap();
    let primary = monitors.iter().find(|monitor| monitor.is_primary).unwrap();
    let abox = AABB::from(primary);

    println!("{:#?}", abox);
}
