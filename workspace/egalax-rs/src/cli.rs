use anyhow::anyhow;
use const_format::formatcp;
use std::{
    fmt,
    fs::File,
    path::{Path, PathBuf},
    process::exit,
};

use crate::error::EgalaxError;

const CONFIG_NAME: &str = "config.toml";
const FALLBACK_CONFIG: &str = formatcp!("./{}", CONFIG_NAME);
const FALLBACK_DEVICE: &str = "/dev/hidraw.egalax";

/// Necessary settings to execute the driver.
#[derive(Debug)]
pub struct ProgramArgs {
    /// Path to the hidraw device.
    device: PathBuf,
    /// Path to the config file.
    config: PathBuf,
}

pub struct ProgramResources {
    /// Hidraw device
    pub device: File,
    /// Config file.
    pub config: File,
}

impl fmt::Display for ProgramArgs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "Hidraw device: {}\nConfig file: {}",
            self.device.display(),
            self.config.display()
        )
    }
}

/// Print CLI usage and then exit with an error.
fn exit_usage() -> ! {
    let program = std::env::args().nth(0).unwrap_or(String::from("<unknown>"));
    let usage = format!("Usage: {} [--dev FILE] [--config FILE]", program);
    eprintln!("{}", usage);
    exit(1)
}

impl ProgramArgs {
    pub fn device(&self) -> &Path {
        &self.device
    }

    pub fn config(&self) -> &Path {
        &self.config
    }

    /// Construct the [`Settings`] by parsing command line arguments and following XDG conventions.
    /// Exits the program if program arguments cannot be parsed correctly.
    pub fn get() -> ProgramArgs {
        // Get command line arguments, skipping the program name.
        let mut args = std::env::args().into_iter().skip(1);
        let mut device: Option<PathBuf> = None;
        let mut config: Option<PathBuf> = None;

        // CLI args have highest precedence.
        while let Some(arg) = args.next() {
            match arg.as_str() {
                "--dev" => {
                    // a.d. Note that we must use the closure `|| exit_usage()` instead of the function pointer `exit_usage`.
                    // While it is possible to coerce ! to any other type, coercions do not work over the structure of types.
                    // So we cannot coerce the function pointer type fn() -> ! to the type fn() -> String (and then on to a closure type).
                    device = Some(args.next().unwrap_or_else(|| exit_usage()).into());
                }
                "--config" => {
                    config = Some(args.next().unwrap_or_else(|| exit_usage()).into());
                }
                _ => {
                    log::error!("Unknown argument: {}", arg);
                    exit_usage();
                }
            }
        }

        // If config was not defined via CLI arg, try to set it via XDG config directory.
        if config.is_none() {
            config = match xdg::BaseDirectories::with_prefix("egalax_rs") {
                Ok(xdg_dirs) => {
                    // First try to find an existing file in XDG_CONFIG_HOME and then XDG_CONFIG_DIRS.
                    xdg_dirs.find_config_file(CONFIG_NAME).or_else(|| {
                        // If it does not exist, we want to place it in XDG_CONFIG_HOME. This may fail if we cannot create the directories.
                        match xdg_dirs.place_config_file(CONFIG_NAME) {
                            Ok(path) => Some(path),
                            Err(e) => {
                                log::warn!("Failed to create config directory: {:?}.", e);
                                None
                            }
                        }
                    })
                }
                Err(e) => {
                    log::warn!("Failed to access XDG directories: {:?}.", e);
                    None
                }
            };
        }

        ProgramArgs {
            device: device.unwrap_or_else(|| {
                log::warn!("Using fallback device path: {}.", FALLBACK_DEVICE);
                FALLBACK_DEVICE.into()
            }),
            config: config.unwrap_or_else(|| {
                log::warn!("Using fallback config path: {}.", FALLBACK_CONFIG);
                FALLBACK_CONFIG.into()
            }),
        }
    }

    pub fn acquire_resources(self) -> Result<ProgramResources, EgalaxError> {
        log::trace!("Entering CLI::get_resources.");
        log::info!("Trying to acquire program resources.");

        let device = File::open(self.device()).map_err(|e| {
            anyhow!(
                "Unable to open hidraw device: {}. USB cable to monitor disconnected?\n\n{}",
                self.device().display(),
                e
            )
        })?;
        log::info!("Opened device node {}.", self.device().display());

        let config = File::open(self.config()).map_err(|e| {
            anyhow!(
                "Unable to open config file: {}.\n{}",
                self.config().display(),
                e
            )
        })?;
        log::info!("Opened config file:\n{}", self.config().display());

        log::trace!("Leaving CLI::get_resources.");
        Ok(ProgramResources { device, config })
    }
}
