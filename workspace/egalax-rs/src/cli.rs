use anyhow::{anyhow, Context};
use const_format::formatcp;
use std::{
    fmt,
    fs::{self, File},
    path::{Path, PathBuf},
    process::exit,
};

use crate::{config::Config, error::EgalaxError};

pub const CONFIG_NAME: &str = "config.toml";
pub const DEFAULT_CONFIG_PATH: &str = formatcp!("/etc/egalax_rs/{}", CONFIG_NAME);
const FALLBACK_DEVICE: &str = "/dev/hidraw.egalax";

/// Necessary settings to execute the driver.
#[derive(Debug)]
pub struct ProgramArgs {
    /// Path to the hidraw device.
    device: PathBuf,
    /// Path to the config file. `None` if no config found (should use [`Config::default`] in that case)
    config: Option<PathBuf>,
}

pub struct ProgramResources {
    /// Hidraw device
    pub device: File,
    /// Config file.
    pub config: Config,
}

impl fmt::Display for ProgramArgs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        writeln!(f, "Hidraw device: {}", self.device.display())?;
        match self.config() {
            Some(config) => write!(f, "Config file: {}", config.display()),
            None => write!(f, "Config file: (default config)"),
        }
    }
}

/// Print CLI usage and then exit with an error.
fn exit_usage() -> ! {
    let program = std::env::args()
        .next()
        .unwrap_or_else(|| String::from("<unknown>"));
    let usage = format!("Usage: {} [--dev FILE] [--config FILE]", program);
    eprintln!("{}", usage);
    exit(1)
}

impl ProgramArgs {
    pub fn device(&self) -> &Path {
        &self.device
    }

    pub fn config(&self) -> Option<&Path> {
        self.config.as_deref()
    }

    /// Construct the [`Settings`] by parsing command line arguments and following XDG conventions.
    /// Exits the program if program arguments cannot be parsed correctly.
    pub fn get() -> Self {
        // Get command line arguments, skipping the program name.
        let mut args = std::env::args().skip(1);
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

        // If config was not defined via CLI arg, try to set it via XDG config directory or `/etc/egalax_rs`.
        if config.is_none() {
            config = match xdg::BaseDirectories::with_prefix("egalax_rs") {
                Ok(xdg_dirs) => {
                    // First try to find an existing file in XDG_CONFIG_HOME and then XDG_CONFIG_DIRS.
                    xdg_dirs.find_config_file(CONFIG_NAME)
                }
                Err(e) => {
                    log::warn!("Failed to access XDG directories: {:?}.", e);
                    None
                }
            };
            config = config.or_else(|| {
                let config_path = PathBuf::from(DEFAULT_CONFIG_PATH);
                if config_path.exists() {
                    Some(config_path)
                } else {
                    // use default config
                    log::warn!("Using default configuration since no config.toml was found");
                    None
                }
            });
        }

        Self {
            device: device.unwrap_or_else(|| {
                log::warn!("Using fallback device path: {}.", FALLBACK_DEVICE);
                FALLBACK_DEVICE.into()
            }),
            config,
        }
    }

    /// Opens the files given in the program arguments
    pub fn acquire_resources(&self) -> Result<ProgramResources, EgalaxError> {
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

        let config = if let Some(config_path) = self.config() {
            let config = fs::read_to_string(config_path)
                .with_context(|| format!("Failed to open config file {}", config_path.display()))?;
            log::info!("Opened config file:\n{}", config_path.display());

            let config = toml::from_str(&config)?;
            log::info!("Using monitor config:\n{}", config);
            config
        } else {
            log::info!("No config file found, using default configuration");
            Config::default()
        };

        log::trace!("Leaving CLI::get_resources.");
        Ok(ProgramResources { device, config })
    }
}
