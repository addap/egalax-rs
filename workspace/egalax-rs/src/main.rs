use std::result::Result;

use egalax_rs::cli::ProgramArgs;
use egalax_rs::config::Config;
use egalax_rs::driver::virtual_mouse;
use egalax_rs::error::EgalaxError;

/// Read configuration and delegate to virtual mouse function.
fn main() -> Result<(), EgalaxError> {
    env_logger::init();

    let args = ProgramArgs::get();
    log::info!("Using arguments:\n{}", args);
    let mut resources = args.acquire_resources()?;

    let monitor_cfg = Config::from_file(&mut resources.config)?;
    log::info!("Using monitor config:\n{}", monitor_cfg);

    virtual_mouse(&mut resources.device, monitor_cfg)?;
    Ok(())
}
