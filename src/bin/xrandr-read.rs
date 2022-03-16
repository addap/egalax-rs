use egalax_rs::config::MonitorConfigBuilder;
use std::error;

fn main() -> Result<(), Box<dyn error::Error>> {
    let touch_monitor_name = String::from("eDP");

    let monitor_cfg = MonitorConfigBuilder::new()?
        .with_name(Some(touch_monitor_name))
        .build()?;

    println!("{:?}", monitor_cfg);

    Ok(())
}
