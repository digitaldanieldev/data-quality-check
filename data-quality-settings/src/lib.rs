// src/lib.rs

use color_eyre::eyre::Error;
use color_eyre::Report;
use dotenvy::from_filename;
use std::env;
use tracing::{debug, error, info, span, Level};
use tracing_subscriber::*;
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    EnvFilter,
};

#[tracing::instrument]
pub fn load_env_variables() {
    info!("load_env_variables");

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

    debug!("Finished loading environment variables.");
}

#[tracing::instrument]
pub fn load_logging_config() -> Result<(), Report> {
    info!("load_logging_config");

    color_eyre::install()?;

    let subscriber = tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .with_span_events(FmtSpan::NONE)
        .event_format(
            fmt::format()
                // .pretty()
                .with_target(false)
                .with_level(true)
                .with_source_location(false),
        )
        .finish();

    tracing::subscriber::set_global_default(subscriber)?;

    Ok(())
}
