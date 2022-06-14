use egalax_rs::config::MonitorConfigBuilder;
use std::error;

fn main() -> Result<(), Box<dyn error::Error>> {
    let monitor_cfg = MonitorConfigBuilder::default().build()?;

    println!("{:?}", monitor_cfg);

    Ok(())
}
