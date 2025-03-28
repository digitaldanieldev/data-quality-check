/* Licensed under the AGPL-3.0 License: https://www.gnu.org/licenses/agpl-3.0.html */

use anyhow::{Error, Result};
use color_eyre::Report;
use dotenvy::from_filename;
use std::env;
use std::fs;
use std::path::Path;
use tracing::{debug, error, info, trace, warn, Level};
use tracing_subscriber::FmtSubscriber;
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    EnvFilter,
};

#[tracing::instrument]
fn is_docker() -> bool {
    trace!("Checking if the application is running inside Docker.");

    if Path::new("/.dockerenv").exists() {
        trace!("Detected Docker environment via /.dockerenv");
        return true;
    }

    if let Ok(content) = fs::read_to_string("/proc/1/cgroup") {
        if content.contains("docker") {
            trace!("Detected Docker environment via /proc/1/cgroup");
            return true;
        }
    }

    trace!("No Docker environment detected.");
    false
}

#[tracing::instrument]
pub fn load_env_variables() {
    trace!("Entering load_env_variables function");

    if is_docker() {
        debug!("Running inside Docker, skipping .env file loading.");
    } else {
        match env::current_exe() {
            Ok(exe_path) => {
                debug!("Current executable path: {:?}", exe_path);

                if let Some(exe_dir) = exe_path.parent() {
                    trace!("Executable directory found: {:?}", exe_dir);

                    let env_file_path = exe_dir.join(".env");
                    debug!("Computed .env file path: {:?}", env_file_path);

                    trace!("Attempting to load .env file at {:?}", env_file_path);
                    match from_filename(env_file_path.to_str().unwrap()) {
                        Ok(_) => {
                            info!(
                                "Environment variables loaded successfully from {:?}",
                                env_file_path
                            );
                        }
                        Err(e) => {
                            error!("Failed to load .env file at {:?}: {}", env_file_path, e);
                        }
                    }

                    match env::var("PROTO_SCHEMA_INPUT_DIR") {
                        Ok(value) => info!("PROTO_SCHEMA_INPUT_DIR: {}", value),
                        Err(_) => {
                            warn!("PROTO_SCHEMA_INPUT_DIR not found in the environment variables.");
                        }
                    }
                } else {
                    error!("Failed to get the executable directory.");
                }
            }
            Err(e) => {
                error!("Failed to get executable path: {}", e);
            }
        }
    }

    debug!("Finished loading environment variables.");
    trace!("Exiting load_env_variables function.");
}

#[tracing::instrument]
pub fn load_logging_config(log_level: Level) -> Result<(), Report> {
    trace!("Entering load_logging_config function");

    color_eyre::install()?;

    let filter = EnvFilter::new(format!("my_crate={}", log_level.as_str()));

    let subscriber = FmtSubscriber::builder()
        .with_env_filter(filter)
        .with_max_level(log_level)
        .with_span_events(FmtSpan::ACTIVE)
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    info!(
        "Logging configuration successfully applied at level: {:?}",
        log_level
    );
    trace!("Exiting load_logging_config function");

    Ok(())
}

pub fn parse_log_level(log_level: &str) -> Result<Level, Error> {
    trace!("Parsing log level: {}", log_level);

    match log_level.to_lowercase().as_str() {
        "error" => Ok(Level::ERROR),
        "warn" => Ok(Level::WARN),
        "info" => Ok(Level::INFO),
        "debug" => Ok(Level::DEBUG),
        "trace" => Ok(Level::TRACE),
        _ => {
            warn!("Unrecognized log level, defaulting to INFO.");
            Ok(Level::INFO)
        }
    }
}
