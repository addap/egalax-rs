use std::{fs::File, io::Write};

use egalax_rs::config::Config;

/// Generate a default config
fn main() -> Result<(), anyhow::Error> {
    let cf = Config::default();
    println!("{:#?}", cf);
    let s = toml::to_string(&cf)?;
    let mut f = File::create("./config.gen.toml")?;
    f.write_all(s.as_bytes())?;
    Ok(())
}
