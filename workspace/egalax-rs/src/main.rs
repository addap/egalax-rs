use std::result::Result;

use egalax_rs::cli::ProgramArgs;
use egalax_rs::driver::virtual_mouse;
use egalax_rs::error::EgalaxError;

/// Read configuration and delegate to virtual mouse function.
fn main() -> Result<(), EgalaxError> {
    env_logger::init();

    let args = ProgramArgs::get();
    log::info!("Using arguments:\n{}", args);
    let mut resources = args.acquire_resources()?;

    virtual_mouse(&mut resources.device, resources.config)?;
    Ok(())
}
