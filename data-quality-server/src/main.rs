use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
    routing::post,
    Router,
};
use base64;
use opentelemetry::{global, metrics::Meter, KeyValue};
use opentelemetry_sdk::metrics::{PeriodicReader, SdkMeterProvider};
use opentelemetry_sdk::runtime::Tokio;
use opentelemetry_sdk::Resource;
use prost_reflect::{
    DescriptorPool, DynamicMessage, Kind, MessageDescriptor, SerializeOptions,
    Value as ProstReflectValue,
};
use prost_types::FileDescriptorSet;
use serde::Deserialize;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::{
    env,
    net::SocketAddr,
    sync::{Arc, Mutex},
};
use tokio::net::TcpListener;
use tracing::{debug, error, info, span, Level};

use data_quality_settings::{load_env_variables, load_logging_config};
use dynamic_message::{populate_dynamic_message, serialize_dynamic_message};

type DescriptorMap = Arc<Mutex<HashMap<String, Vec<u8>>>>;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    load_logging_config();
    load_env_variables();
    let _meter_provider = init_meter_provider();

    let dq_server_port = env::var("DQ_SERVER_PORT").unwrap_or_else(|_| "8080".to_string());
    let port: u64 = dq_server_port.parse()?;

    let descriptor_map = Arc::new(Mutex::new(HashMap::new()));

    let app = Router::new()
        .route("/load_descriptor", post(load_descriptor_handler))
        .route("/validate", post(validate_json_handler))
        .with_state(descriptor_map);

    let addr = format!("0.0.0.0:{}", port).parse::<SocketAddr>()?;
    info!(
        "Listening for descriptor loading and JSON validation on {:?}",
        addr
    );

    let listener = TcpListener::bind(addr).await.unwrap();
    info!("Starting server on port {}", port);

    axum::serve(listener, app.into_make_service()).await?;

    Ok(())
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
    State(descriptor_map): State<DescriptorMap>,
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
        let mut descriptor_map = descriptor_map.lock().unwrap();
        descriptor_map.insert(file_name.clone(), file_content.clone());
    }

    let new_descriptor_pool = match rebuild_descriptor_pool(&descriptor_map.lock().unwrap()) {
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

////////////////
// validation //
////////////////

#[derive(Deserialize)]
struct ValidationRequest {
    n: Option<String>,
    json: String,
}

async fn validate_json_handler(
    State(descriptor_map): State<DescriptorMap>, // Updated type to match the new `DescriptorMap`
    Json(payload): Json<ValidationRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let span = span!(Level::INFO, "validate_json_handler");
    let _enter = span.enter();

    let proto_name = payload.n.unwrap_or_else(|| "MyMessage".to_string());
    let json_message = payload.json;

    let descriptor_pool = {
        let descriptor_map = descriptor_map.lock().unwrap();
        match rebuild_descriptor_pool(&descriptor_map) {
            Ok(pool) => pool,
            Err(err) => {
                error!("Failed to rebuild descriptor pool: {}", err);
                return Err(StatusCode::INTERNAL_SERVER_ERROR);
            }
        }
    };

    match validate_json_against_proto(&descriptor_pool, &json_message, &proto_name) {
        Ok(_) => Ok((StatusCode::OK, "JSON validation successful".to_string())),
        Err(e) => {
            error!("JSON validation failed: {}", e);
            Err(StatusCode::BAD_REQUEST)
        }
    }
}

#[tracing::instrument]
fn validate_json_against_proto(
    descriptor_pool: &DescriptorPool,
    json_message: &str,
    definition_name: &str,
) -> Result<(), String> {
    let meter = global::meter("json-validation-service");
    let (request_counter, duration_histogram) = create_metrics(&meter);

    let start_time = std::time::Instant::now();

    request_counter.add(
        1,
        &[KeyValue::new("message_name", definition_name.to_string())],
    );

    info!("Starting JSON validation for proto: {}", definition_name);

    let message_descriptor = descriptor_pool
        .get_message_by_name(definition_name)
        .ok_or_else(|| {
            let error_msg = format!("Message '{}' not found in pool", definition_name);
            error!("{}", error_msg);
            error_msg
        })?;

    info!("Found message descriptor: {:?}", message_descriptor);

    let json_value: JsonValue = serde_json::from_str(json_message).map_err(|e| {
        let error_msg = format!("Failed to parse JSON: {:?}", e);
        error!("{}", error_msg);
        error_msg
    })?;

    let mut dynamic_message = DynamicMessage::new(message_descriptor.clone());
    populate_dynamic_message(&mut dynamic_message, &message_descriptor, &json_value).map_err(
        |e| {
            let error_msg = format!("Failed to populate dynamic message: {}", e);
            error!("{}", error_msg);
            error_msg
        },
    )?;

    serialize_dynamic_message(&mut dynamic_message).map_err(|e| {
        let error_msg = format!("Failed to serialize dynamic message: {}", e);
        error!("{}", error_msg);
        error_msg
    })?;

    let duration = start_time.elapsed().as_secs_f64();
    duration_histogram.record(
        duration,
        &[KeyValue::new("message_name", definition_name.to_string())],
    );

    Ok(())
}
