#![warn(clippy::pedantic)]
#![allow(
    clippy::must_use_candidate,
    clippy::uninlined_format_args,
    clippy::missing_errors_doc,
    clippy::explicit_iter_loop
)]

mod app;

use std::path::{Path, PathBuf};

use thiserror::Error;

use app::App;
use egalax_rs::cli::{ProgramArgs, ProgramResources, CONFIG_NAME};

/// Combination of errors from our driver of the GUI framework.
#[derive(Debug, Error)]
enum Error {
    #[error("{0}")]
    Eframe(#[from] eframe::Error),
    #[error("{0}")]
    Egalax(#[from] egalax_rs::error::EgalaxError),
}

fn main() -> Result<(), Error> {
    env_logger::init();

    let args = ProgramArgs::get();
    log::info!("Using arguments:\n{}", args);

    let device_path = args.device().to_path_buf();
    let config_path = args
        .config()
        .map_or_else(|| PathBuf::from(CONFIG_NAME), Path::to_path_buf);
    // a.d. TODO can we change it to keep both files open? iirc I tried it but when I tried to save the config I received a "bad file descriptor" error.
    let ProgramResources { device, config } = args.acquire_resources()?;

    log::info!("Tested opening device node.");
    drop(device);

    eframe::run_native(
        "egalax settings editor",
        eframe::NativeOptions::default(),
        Box::new(|cc| Ok(Box::new(App::new(device_path, config_path, config, cc)))),
    )?;
    Ok(())
}
