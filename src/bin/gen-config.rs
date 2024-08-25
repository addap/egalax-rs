use std::{fs::OpenOptions, io::Write};

use egalax_rs::config::ConfigFile;

/// Generate a default config
fn main() -> Result<(), anyhow::Error> {
    let cf = ConfigFile::default();
    println!("{:#?}", cf);
    let s = toml::to_string(&cf)?;
    let mut f = OpenOptions::new()
        .write(true)
        .create(true)
        .open("./config.toml")?;
    f.write_all(s.as_bytes())?;
    Ok(())
}
