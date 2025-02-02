//! Used to check the default monitor config that we generate.

use std::error;

use egalax_rs::config::SerializedConfig;

fn main() -> Result<(), Box<dyn error::Error>> {
    let monitor_cfg = SerializedConfig::default().build()?;

    println!("{}", monitor_cfg);

    Ok(())
}
