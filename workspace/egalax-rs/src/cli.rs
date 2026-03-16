use anyhow::Context;
use clap::Parser;
use const_format::formatcp;
use std::{
    fmt,
    fs::{self, File},
    path::{Path, PathBuf},
    str::FromStr,
};

use crate::config::Config;

pub const CONFIG_PREFIX: &str = "egalax_rs";
pub const CONFIG_NAME: &str = "config.toml";
pub const DEFAULT_CONFIG_PATH: &str = formatcp!("/etc/{CONFIG_PREFIX}/{CONFIG_NAME}",);

fn fallback_config_path() -> Option<PathBuf> {
    // If config was not defined via CLI arg, try to set it via XDG config directory or `/etc/egalax_rs`.
    let config = match xdg::BaseDirectories::with_prefix(CONFIG_PREFIX) {
        Ok(xdg_dirs) => {
            // First try to find an existing file in XDG_CONFIG_HOME and then XDG_CONFIG_DIRS.
            xdg_dirs.find_config_file(CONFIG_NAME)
        }
        Err(e) => {
            log::warn!("Failed to access XDG directories: {:?}.", e);
            None
        }
    };
    let config = config.or_else(|| {
        let config_path = PathBuf::from(DEFAULT_CONFIG_PATH);
        if config_path.exists() {
            Some(config_path)
        } else {
            // use default config
            log::warn!("Using default configuration since no config.toml was found");
            None
        }
    });
    config
}

/// Userspace driver for Egalax Touchscreens.
#[derive(Parser, Debug)]
pub struct ProgramArgs {
    /// Path to the hidraw device.
    #[arg(short, long)]
    device: PathBuf,
    /// Path to the config file. When not given, falls back to
    /// - `XDG_CONFIG_HOME/egalax_rs/config.toml`
    /// - `[every dir in XDG_CONFIG_DIRS]/egalax_rs/config.toml`
    /// - `/etc/egalax_rs/config.toml`
    /// - hardcoded defaults
    #[arg(short, long, verbatim_doc_comment)]
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

impl ProgramArgs {
    pub fn device(&self) -> &Path {
        &self.device
    }

    pub fn config(&self) -> Option<&Path> {
        self.config.as_deref()
    }

    /// Opens the files given in the program arguments
    pub fn acquire_resources(self) -> anyhow::Result<ProgramResources> {
        log::trace!("Entering CLI::get_resources.");
        log::info!("Trying to acquire program resources.");

        let device = File::open(self.device()).with_context(|| {
            format!(
                "Unable to open hidraw device: {}. USB cable to monitor disconnected?",
                self.device().display(),
            )
        })?;
        log::info!("Opened device node {}.", self.device().display());

        // fall back to standard config paths...
        let config = self.config.or_else(fallback_config_path);

        let config = if let Some(config_path) = config {
            let config = fs::read_to_string(&config_path)
                .with_context(|| format!("Failed to open config file {}", config_path.display()))?;
            log::info!("Opened config file:\n{}", config_path.display());

            let config = Config::from_str(&config).with_context(|| {
                format!("Failed to parse config file {}", config_path.display())
            })?;
            config
        } else {
            // ...or to a hardcoded config
            log::info!("No config file found, using default configuration");
            Config::default()
        };
        log::info!("Using monitor config:\n{}", config);

        log::trace!("Leaving CLI::get_resources.");
        Ok(ProgramResources { device, config })
    }
}
