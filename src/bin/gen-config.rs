use std::{error, fs::OpenOptions, io::Write};

use egalax_rs::config::MonitorConfigBuilder;
use serde_lexpr;

/// Generate a default config
fn main() -> Result<(), Box<dyn error::Error>> {
    let cf = MonitorConfigBuilder::default();
    println!("{:#?}", cf);
    let s = serde_lexpr::to_string(&cf)?;
    let mut f = OpenOptions::new()
        .write(true)
        .create(true)
        .open("./config")?;
    f.write_all(s.as_bytes())?;
    Ok(())
}
