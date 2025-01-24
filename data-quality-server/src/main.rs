use anyhow::{anyhow, Context, Result};
use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Router,
};
use base64;
use clap::{Parser, ArgAction};
use opentelemetry::{global, metrics::Meter, KeyValue};
use opentelemetry_sdk::metrics::{PeriodicReader, SdkMeterProvider};
use opentelemetry_sdk::runtime::Tokio;
use opentelemetry_sdk::Resource;
use prost_reflect::{DescriptorPool, DynamicMessage};
use prost_types::FileDescriptorSet;
use serde::Deserialize;
use serde_json::{json, Value as JsonValue};
use std::{collections::HashMap, time::Instant};
use std::{
    env,
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use thiserror::Error;
use tokio::net::TcpListener;
use tokio::runtime::Builder;
use tracing::{debug, error, info, span, Level};

use data_quality_settings::{load_env_variables, load_logging_config};
use dynamic_message::{populate_dynamic_message, serialize_dynamic_message};

type DescriptorMap = Arc<Mutex<HashMap<String, Vec<u8>>>>;

#[derive(Clone)]
struct AppState {
    descriptor_map: DescriptorMap,
    enable_metrics: bool,
}

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Failed to load descriptor")]
    LoadDescriptorError(#[source] std::io::Error),

    #[error("Failed to parse JSON: {0}")]
    JsonParseError(#[source] serde_json::Error),

    #[error("Missing environment variable: {0}")]
    MissingEnvVarError(String),

    #[error("Unknown error occurred: {0}")]
    UnknownError(String),
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

        // If `log_level` argument is provided, set level
        let log_level = match cli_args.log_level.to_lowercase().as_str() {
            "error" => Level::ERROR,
            "warn" => Level::WARN,
            "info" => Level::INFO,
            "debug" => Level::DEBUG,
            "trace" => Level::TRACE,
            _ => Level::INFO,
        };

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

////////////////
// validation //
////////////////

#[derive(Deserialize)]
struct ValidationRequest {
    protobuf: Option<String>,
    json: String,
    field_check: Option<bool>,
    field_name: Option<String>,
    field_value_check: Option<JsonValue>,
}

async fn validate_json_handler(
    State(state): State<AppState>,
    Json(payload): Json<ValidationRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let span = span!(Level::INFO, "validate_json_handler");
    let _enter = span.enter();

    let proto_name = payload.protobuf;
    let json_message = payload.json;

    let descriptor_pool = {
        let descriptor_map = state.descriptor_map.lock().unwrap();
        match rebuild_descriptor_pool(&descriptor_map) {
            Ok(pool) => pool,
            Err(err) => {
                error!("Failed to rebuild descriptor pool: {}", err);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    };

    let enable_metrics = state.enable_metrics;

    match validate_json(
        Some(&descriptor_pool),
        &json_message,
        proto_name.as_deref(),
        payload.field_check,
        payload.field_name,
        payload.field_value_check,
        enable_metrics,
    ) {
        Ok(_) => Ok((StatusCode::OK, Json(json!({ "message": "Valid JSON" })))),
        Err(e) => {
            error!("JSON validation failed: {}", e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

#[tracing::instrument]
fn validate_json(
    descriptor_pool: Option<&DescriptorPool>,
    json_message: &str,
    definition_name: Option<&str>,
    field_check: Option<bool>,
    field_name: Option<String>,
    field_value_check: Option<JsonValue>,
    enable_metrics: bool,
) -> Result<(), anyhow::Error> {
    info!("Starting JSON validation process");

    // Metrics setup (if enabled)
    let meter = if enable_metrics {
        Some(global::meter("json-validation-service"))
    } else {
        None
    };

    let start_time = meter.as_ref().map(|_| Instant::now());

    // Parse the JSON first
    let json_value: JsonValue = serde_json::from_str(json_message).map_err(|e| {
        let error_msg = format!("Failed to parse JSON: {:?}", e);
        error!("{}", error_msg);
        anyhow::anyhow!(error_msg)
    })?;

    // Determine the message name for metrics
    let message_name = definition_name.unwrap_or("only_json").to_string();

    // Metrics: Record request count
    if let Some(ref meter) = meter {
        let (request_counter, _) = create_metrics(meter);
        request_counter.add(
            1,
            &[
                KeyValue::new("message_name", message_name.clone()),
                KeyValue::new(
                    "field_check",
                    if field_check.unwrap_or(false) {
                        "enabled"
                    } else {
                        "disabled"
                    },
                ),
            ],
        );
    }

    // Metrics: Track duration
    let record_duration = |message_name: &str, field_check_enabled: bool| {
        if let (Some(start_time), Some(ref meter)) = (start_time, &meter) {
            let duration = start_time.elapsed().as_micros();
            let (_, duration_histogram) = create_metrics(meter);
            let formatted_duration = format!("{:.6}", duration);

            duration_histogram.record(
                formatted_duration.parse().unwrap_or(0.0),
                &[
                    KeyValue::new("message_name", message_name.to_string()),
                    KeyValue::new(
                        "field_check",
                        if field_check_enabled {
                            "enabled"
                        } else {
                            "disabled"
                        },
                    ),
                ],
            );
        }
    };

    // Handle JSON validation with or without a definition name
    if let Some(definition_name) = definition_name {
        info!("Starting JSON validation for proto: {}", definition_name);

        let message_descriptor = descriptor_pool
            .ok_or_else(|| {
                let error_msg = "Descriptor pool is None".to_string();
                error!("{}", error_msg);
                anyhow::anyhow!(error_msg)
            })?
            .get_message_by_name(definition_name)
            .ok_or_else(|| {
                let error_msg = format!("Message '{}' not found in pool", definition_name);
                error!("{}", error_msg);
                anyhow::anyhow!(error_msg)
            })?;

        info!("Found message descriptor: {:?}", message_descriptor);

        // Populate and serialize the dynamic message
        let mut dynamic_message = DynamicMessage::new(message_descriptor.clone());
        populate_dynamic_message(&mut dynamic_message, &message_descriptor, &json_value).map_err(
            |e| {
                let error_msg = format!("Failed to populate dynamic message: {}", e);
                error!("{}", error_msg);
                anyhow::anyhow!(error_msg)
            },
        )?;

        serialize_dynamic_message(&mut dynamic_message).map_err(|e| {
            let error_msg = format!("Failed to serialize dynamic message: {}", e);
            error!("{}", error_msg);
            anyhow::anyhow!(error_msg)
        })?;

        // Perform field validation if enabled
        if field_check.unwrap_or(false) {
            validate_json_message_content(&json_value, field_name, field_value_check).map_err(
                |e| {
                    let error_msg = format!("Failed to validate message content: {}", e);
                    error!("{}", error_msg);
                    anyhow::anyhow!(error_msg)
                },
            )?;
        }

        record_duration(&message_name, field_check.unwrap_or(false));
    } else {
        // Handle "only_json" case
        info!("No definition_name provided. Only parsed JSON successfully.");

        if field_check.unwrap_or(false) {
            validate_json_message_content(&json_value, field_name, field_value_check).map_err(
                |e| {
                    let error_msg = format!("Failed to validate message content: {}", e);
                    error!("{}", error_msg);
                    anyhow::anyhow!(error_msg)
                },
            )?;
        }

        record_duration("only_json", field_check.unwrap_or(false));
    }

    Ok(())
}

fn validate_json_message_content(
    json_value: &JsonValue,
    field_name: Option<String>,
    field_value_check: Option<JsonValue>,
) -> Result<(), String> {
    if let (Some(field), Some(expected_value)) = (field_name, field_value_check) {
        if let Some(actual_value) = json_value.get(&field) {
            Ok(if actual_value != &expected_value {
                let error_msg = format!(
                    "Field '{}' value mismatch: expected {:?}, found {:?}",
                    field, expected_value, actual_value
                );
                error!("{}", error_msg);
                return Err(error_msg);
            })
        } else {
            let error_msg = format!("Field '{}' not found in the JSON", field);
            error!("{}", error_msg);
            return Err(error_msg);
        }
    } else {
        let error_msg = "Field name and value must be provided for validation".to_string();
        error!("{}", error_msg);
        return Err(error_msg);
    }
}

///////////////////
// opentelemetry //
///////////////////

fn create_metrics(
    meter: &Meter,
) -> (
    opentelemetry::metrics::Counter<u64>,
    opentelemetry::metrics::Histogram<f64>,
) {
    let request_counter = meter
        .u64_counter("validate_json_requests_total")
        .with_description("Counts the total number of JSON validation requests")
        .build();

    let duration_histogram = meter
        .f64_histogram("validate_json_duration_seconds")
        .with_description("Tracks the duration of JSON validation in seconds")
        .build();

    (request_counter, duration_histogram)
}

fn init_meter_provider() -> SdkMeterProvider {
    let exporter = opentelemetry_stdout::MetricExporterBuilder::default().build();
    let reader = PeriodicReader::builder(exporter, Tokio).build();
    let resource = Resource::new(vec![KeyValue::new(
        "service.name",
        "json-validation-service",
    )]);
    let provider = SdkMeterProvider::builder()
        .with_reader(reader)
        .with_resource(resource)
        .build();
    global::set_meter_provider(provider.clone());
    provider
}

/////////////////////
// load descriptor //
/////////////////////

#[tracing::instrument]
fn load_descriptor(
    descriptor_pool: &mut DescriptorPool,
    filename: &str,
    proto_content: &[u8],
) -> Result<(), String> {
    info!("load_descriptor: {}", filename);

    let file_descriptor_set: FileDescriptorSet =
        prost::Message::decode(proto_content).map_err(|e| {
            error!(
                "Failed to parse .proto definition for {}: {:?}",
                filename, e
            );
            format!(
                "Failed to parse .proto definition for {}: {:?}",
                filename, e
            )
        })?;

    descriptor_pool
        .add_file_descriptor_set(file_descriptor_set)
        .map_err(|e| {
            error!(
                "Failed to add file descriptor to pool ({}): {:?}",
                filename, e
            );
            format!(
                "Failed to add file descriptor to pool ({}): {:?}",
                filename, e
            )
        })?;

    info!("Successfully loaded descriptor from file: {}", filename);
    Ok(())
}

#[tracing::instrument]
fn load_descriptors(
    descriptor_pool: &mut DescriptorPool,
    files: Vec<(String, Vec<u8>)>,
) -> Result<(), String> {
    info!("load_descriptors");

    let mut failed_files = Vec::new();

    for (filename, proto_content) in files {
        debug!("Processing file: {}", filename);

        if let Err(err) = load_descriptor(descriptor_pool, &filename, &proto_content) {
            error!("Error loading file {}: {}", filename, err);
            failed_files.push(filename);
        } else {
            debug!("Successfully processed file: {}", filename);
        }
    }

    if !failed_files.is_empty() {
        let failed_files_list = failed_files.join(", ");
        error!(
            "Failed to load descriptors for the following files: {}",
            failed_files_list
        );
    } else {
        info!("All files successfully loaded into the descriptor pool.");
    }

    Ok(())
}

#[derive(Deserialize)]
struct LoadDescriptorRequest {
    file_name: String,
    file_content: String,
}

async fn load_descriptor_handler(
    State(state): State<AppState>,
    Json(payload): Json<LoadDescriptorRequest>,
) -> impl IntoResponse {
    let span = span!(Level::INFO, "load_descriptor_handler");
    let _enter = span.enter();

    let file_name = payload.file_name;
    let file_content_base64 = payload.file_content;

    let file_content = match base64::decode(&file_content_base64) {
        Ok(decoded) => decoded,
        Err(err) => {
            error!("Failed to decode base64 content for {}: {}", file_name, err);
            return (
                StatusCode::BAD_REQUEST,
                format!("Failed to decode file content: {}", err),
            )
                .into_response();
        }
    };

    {
        let mut descriptor_map = state.descriptor_map.lock().unwrap();
        descriptor_map.insert(file_name.clone(), file_content.clone());
    }

    let new_descriptor_pool = match rebuild_descriptor_pool(&state.descriptor_map.lock().unwrap()) {
        Ok(pool) => pool,
        Err(err) => {
            error!("Failed to rebuild descriptor pool: {}", err);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to rebuild descriptor pool: {}", err),
            )
                .into_response();
        }
    };

    info!("Descriptor {} loaded successfully.", file_name);
    (
        StatusCode::OK,
        format!("Descriptor {} loaded successfully.", file_name),
    )
        .into_response()
}

fn rebuild_descriptor_pool(
    descriptor_map: &HashMap<String, Vec<u8>>,
) -> Result<DescriptorPool, String> {
    let mut descriptor_pool = DescriptorPool::default();

    for (file_name, file_content) in descriptor_map {
        let file_descriptor_set: FileDescriptorSet =
            prost::Message::decode(file_content.as_slice()).map_err(|e| {
                let error_msg = format!("Failed to parse descriptor {}: {:?}", file_name, e);
                error!("{}", error_msg);
                error_msg
            })?;

        descriptor_pool
            .add_file_descriptor_set(file_descriptor_set)
            .map_err(|e| {
                let error_msg = format!("Failed to add descriptor {}: {:?}", file_name, e);
                error!("{}", error_msg);
                error_msg
            })?;
    }

    Ok(descriptor_pool)
}
