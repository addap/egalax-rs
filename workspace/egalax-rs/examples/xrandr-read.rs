//! Used to check the default monitor config that we generate.

use std::error;

use egalax_rs::config::ConfigFile;

fn main() -> Result<(), Box<dyn error::Error>> {
    let monitor_cfg = ConfigFile::default().build()?;

    println!("{}", monitor_cfg);

    Ok(())
}
