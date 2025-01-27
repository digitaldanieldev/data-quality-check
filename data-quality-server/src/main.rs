use crate::handlers::{load_descriptor_handler, validate_json_handler};
use anyhow::{anyhow, Context, Result};
use axum::{routing::post, Router};
use clap::Parser;
use json_validation::validate_json;
use metrics::init_meter_provider;
use std::collections::HashMap;
use std::{
    env,
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tokio::net::TcpListener;
use tokio::runtime::Builder;
use tracing::{debug, error, info, span, Level};

use data_quality_settings::{load_env_variables, load_logging_config, parse_log_level};

pub mod app_error;
pub mod handlers;
pub mod json_validation;
pub mod metrics;
pub mod protobuf_descriptors;

type DescriptorMap = Arc<Mutex<HashMap<String, Vec<u8>>>>;

#[derive(Clone)]
pub struct AppState {
    descriptor_map: DescriptorMap,
    enable_metrics: bool,
}
#[derive(Parser, Debug)]
#[command(version, about = "Proto Producer", long_about = None)]
struct Args {
    /// Enable metrics
    #[arg(long, action(clap::ArgAction::SetTrue))]
    enable_metrics: bool,

    /// Optional JSON string to validate json
    #[clap(short, long)]
    json: Option<String>,

    /// Number of worker threads
    #[clap(long, default_value_t = 2)]
    worker_threads: usize,

    /// Logging level
    #[clap(short, long, default_value = "info")]
    log_level: String,
}

fn main() -> Result<(), anyhow::Error> {
    let cli_args: Args = Args::parse();

    // Dynamically configure the Tokio runtime with the specified number of worker threads
    let runtime = Builder::new_multi_thread()
        .worker_threads(cli_args.worker_threads)
        .enable_all()
        .build()?;

    // Enter the Tokio runtime
    runtime.block_on(async {
        // If `json` argument is provided, validate JSON
        if let Some(json_string) = cli_args.json {
            match validate_json(
                None,
                &json_string,
                None,
                Some(false),
                None,
                None,
                cli_args.enable_metrics,
            ) {
                Ok(_) => {
                    println!("JSON OK");
                    return Ok(());
                }
                Err(e) => {
                    error!("JSON validation failed: {}", e);
                    return Err(anyhow!("Validation failed for the provided JSON"));
                }
            }
        }

        let log_level = parse_log_level(&cli_args.log_level)?;
        let _ = load_logging_config(log_level);
        load_env_variables();

        let _meter_provider = if cli_args.enable_metrics {
            Some(init_meter_provider())
        } else {
            None
        };
        let server_ip = env::var("SERVER_IP").context("SERVER_IP environment variable missing")?;
        let server_port =
            env::var("SERVER_PORT").context("SERVER_PORT environment variable missing")?;

        let server_address = format!("{}:{}", server_ip, server_port);

        let app_state = AppState {
            descriptor_map: Arc::new(Mutex::new(HashMap::new())),
            enable_metrics: cli_args.enable_metrics,
        };

        let app = Router::new()
            .route("/load_descriptor", post(load_descriptor_handler))
            .route("/validate", post(validate_json_handler))
            .with_state(app_state);

        let tcp_listener_address: SocketAddr = format!("{}", server_address)
            .parse::<SocketAddr>()
            .map_err(|e| anyhow::anyhow!("Failed to parse SocketAddr: {}", e))?; // Use anyhow for error handling

        info!(
            "Listening for descriptor loading and JSON validation on {:?}",
            tcp_listener_address
        );

        let listener = TcpListener::bind(tcp_listener_address)
            .await
            .context("Failed to bind TcpListener")?;

        info!("Starting server on port {}", server_port);

        axum::serve(listener, app.into_make_service()).await?;

        Ok(())
    })
}
