#![warn(clippy::pedantic)]
#![allow(
    clippy::must_use_candidate,
    clippy::uninlined_format_args,
    clippy::missing_errors_doc,
    clippy::explicit_iter_loop
)]

mod app;

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context};
use thiserror::Error;

use app::App;
use egalax_rs::cli::{ProgramArgs, ProgramResources, DEFAULT_CONFIG_PATH};

/// Combination of errors from our driver of the GUI framework.
#[derive(Debug, Error)]
enum Error {
    #[error("{0}")]
    Eframe(#[from] eframe::Error),
    #[error("{0}")]
    Egalax(#[from] egalax_rs::error::EgalaxError),
    #[error("{0}")]
    Generic(#[from] anyhow::Error),
}

fn main() -> Result<(), Error> {
    env_logger::init();

    if std::env::args()
        .nth(1)
        .is_some_and(|x| x == "--apply-config")
    {
        // called by the config after escalating privileges to copy the config to the right folder
        let config_path = std::env::args_os()
            .nth(2)
            .context("expecting 2 arguments")?;
        let config = std::env::args_os()
            .nth(3)
            .context("expecting 2 arguments")?
            .into_string()
            .map_err(|_| anyhow!("invalid config string"))?;
        fs::create_dir_all(
            PathBuf::from(&config_path)
                .parent()
                .context("invalid path")?,
        )
        .context("failed to create config directories")?;
        fs::write(config_path, config).context("failed to write config file")?;
        return Ok(());
    }

    let args = ProgramArgs::get();
    log::info!("Using arguments:\n{}", args);

    let device_path = args.device().to_path_buf();
    let config_path = args
        .config()
        .map_or_else(|| PathBuf::from(DEFAULT_CONFIG_PATH), Path::to_path_buf);
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
