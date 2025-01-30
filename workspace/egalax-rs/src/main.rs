use std::fmt;
use std::fs::OpenOptions;
use std::path::PathBuf;
use std::process::exit;
use std::result::Result;

use const_format::formatcp;
use egalax_rs::config::ConfigFile;
use egalax_rs::driver::virtual_mouse;
use egalax_rs::error::EgalaxError;

const DEVICE_DEFAULT: &str = "/dev/hidraw.egalax";
const CONFIG_NAME: &str = "config.toml";
const CONFIG_FALLBACK: &str = formatcp!("./{}", CONFIG_NAME);
const USAGE: &str = "Usage: egalax-rs [--dev FILE] [--config FILE]";

/// Necessary settings to execute the driver.
#[derive(Debug)]
struct Settings {
    /// Path to the hidraw device.
    device: PathBuf,
    /// Path to the config file location.
    config: PathBuf,
}

impl fmt::Display for Settings {
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
    log::error!("{}", USAGE);
    exit(1)
}

/// Construct the [`Settings`] by parsing command line arguments and following XDG conventions.
fn get_settings() -> Settings {
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

    Settings {
        device: device.unwrap_or(DEVICE_DEFAULT.into()),
        config: config.unwrap_or_else(|| {
            log::warn!("Using fallback config path: {}.", CONFIG_FALLBACK);
            CONFIG_FALLBACK.into()
        }),
    }
}

/// Read configuration and delegate to virtual mouse function.
fn main() -> Result<(), EgalaxError> {
    env_logger::init();

    let settings = get_settings();

    log::info!("Using settings:\n{}", settings);

    let mut device_node = OpenOptions::new()
        .read(true)
        .open(&settings.device)
        .expect(&format!(
            "Unable to open hidraw device: {}.",
            settings.device.display()
        ));
    log::info!("Opened device node {}.", settings.device.display());

    let monitor_cfg = ConfigFile::from_file(&settings.config)?.build()?;
    log::info!("Using monitor config:\n{}", monitor_cfg);

    virtual_mouse(&mut device_node, monitor_cfg)?;
    Ok(())
}
