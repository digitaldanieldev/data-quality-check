// src/lib.rs

use anyhow::Result;
use color_eyre::Report;
use dotenvy::from_filename;
use std::env;
use tracing::{Level, info, error, warn, debug, trace};
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    EnvFilter,
};
use tracing_subscriber::FmtSubscriber;

#[tracing::instrument]
pub fn load_env_variables() {
    info!("load_env_variables");

    if is_docker() {
        debug!("Running inside Docker, skipping .env file loading.");
    } else {
        match env::current_exe() {
            Ok(exe_path) => {
                debug!("Current executable path: {:?}", exe_path);

                if let Some(exe_dir) = exe_path.parent() {
                    let env_file_path = exe_dir.join(".env");
                    debug!("Computed .env file path: {:?}", env_file_path);

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
                            error!("PROTO_SCHEMA_INPUT_DIR not found in the environment variables.");
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
}

#[tracing::instrument]
pub fn load_logging_config(log_level: Level) -> Result<(), Report> {
    tracing::info!("load_logging_config");

    color_eyre::install()?;

    let subscriber = FmtSubscriber::builder()
        .with_max_level(log_level)
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    Ok(())
}

#[tracing::instrument]
pub fn parse_log_level(log_level: &str) -> Result<Level, anyhow::Error> {
    match log_level.to_lowercase().as_str() {
        "error" => Ok(Level::ERROR),
        "warn" => Ok(Level::WARN),
        "info" => Ok(Level::INFO),
        "debug" => Ok(Level::DEBUG),
        "trace" => Ok(Level::TRACE),
        _ => Ok(Level::INFO),
    }
}

use std::fs;
use std::path::Path;

#[tracing::instrument]
fn is_docker() -> bool {
    if Path::new("/.dockerenv").exists() {
        return true;
    }

    if let Ok(content) = fs::read_to_string("/proc/1/cgroup") {
        if content.contains("docker") {
            return true;
        }
    }

    false
}
