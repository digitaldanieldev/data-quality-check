use axum::{extract::{Json, State}, http::StatusCode, response::IntoResponse, routing::post, Router};
use base64;
use prost::Message;
use prost_reflect::{DescriptorPool, DynamicMessage, Kind, MessageDescriptor, SerializeOptions, Value as ProstReflectValue};
use prost_types::FileDescriptorSet;
use serde::Deserialize;
use serde_json::Value as JsonValue;
use std::{env, fs::File, io::{self, Read}, net::SocketAddr, sync::{Arc, Mutex}};
use tokio::net::TcpListener;
use tracing::{debug, error, info, span, Level};

use data_quality_settings::{load_env_variables, load_logging_config};


#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = load_logging_config();
    load_env_variables();

    let dq_server_port = env::var("DQ_SERVER_PORT").unwrap_or_else(|_| "8080".to_string());
    let port: u64 = dq_server_port.parse()?;

    let descriptor_pool = Arc::new(Mutex::new(DescriptorPool::default()));

    let app = Router::new()
        .route("/load_descriptor", post(load_descriptor_handler))
        .route("/validate", post(validate_json_handler))
        .with_state(descriptor_pool);

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

// Struct for loading descriptors
#[derive(Deserialize)]
struct LoadDescriptorRequest {
    file_name: String,
    file_content: String,
}

// Struct for validating JSON against descriptors
#[derive(Deserialize)]
struct ValidationRequest {
    n: Option<String>,
    json: String,
}

async fn load_descriptor_handler(
    State(descriptor_pool): State<Arc<Mutex<DescriptorPool>>>,
    Json(payload): Json<LoadDescriptorRequest>,
) -> impl IntoResponse {
    let span = span!(Level::INFO, "load_descriptor_handler");
    let _enter = span.enter();

    let file_name = payload.file_name;
    let file_content_base64 = payload.file_content;

    // Decode the base64 encoded content back to bytes
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

    // Lock the descriptor pool to mutate it
    let mut descriptor_pool = descriptor_pool.lock().unwrap();

    // Load the descriptor (dummy implementation)
    if let Err(e) = load_descriptor(&mut descriptor_pool, &file_name, &file_content) {
        error!("Failed to load descriptor {}: {}", file_name, e);
        return (
            StatusCode::BAD_REQUEST,
            format!("Failed to load descriptor: {}", e),
        )
            .into_response();
    }

    info!("Descriptor {} loaded successfully.", file_name);
    (
        StatusCode::OK,
        format!("Descriptor {} loaded successfully.", file_name),
    )
        .into_response()
}

async fn validate_json_handler(
    State(descriptor_pool): State<Arc<Mutex<DescriptorPool>>>,
    Json(payload): Json<ValidationRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let span = span!(Level::INFO, "validate_json_handler");
    let _enter = span.enter();

    let proto_name = payload.n.unwrap_or_else(|| "MyMessage".to_string());
    let json_message = payload.json;

    let descriptor_pool = descriptor_pool.lock().unwrap();

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
    info!("Starting JSON validation for proto: {}", definition_name);

    // Attempt to find the message descriptor by name
    let message_descriptor = descriptor_pool
        .get_message_by_name(definition_name)
        .ok_or_else(|| {
            let error_msg = format!("Message '{}' not found in pool", definition_name);
            error!("{}", error_msg);
            error_msg
        })?;

    // Log the message descriptor details
    info!("Found message descriptor: {:?}", message_descriptor);

    // Parse and validate the JSON message
    let json_value: JsonValue =
        serde_json::from_str(json_message).map_err(|e| {
            let error_msg = format!("Failed to parse JSON: {:?}", e);
            error!("{}", error_msg);
            error_msg
        })?;

    info!("Successfully parsed JSON: {:?}", json_value);

    let mut dynamic_message = DynamicMessage::new(message_descriptor.clone());
    match populate_dynamic_message(&mut dynamic_message, &message_descriptor, &json_value) {
        Ok(_) => info!("Populated dynamic message successfully"),
        Err(e) => {
            let error_msg = format!("Failed to populate dynamic message: {}", e);
            error!("{}", error_msg);
            return Err(error_msg);
        }
    }

    match serialize_dynamic_message(&mut dynamic_message) {
        Ok(_) => info!("Serialized dynamic message successfully"),
        Err(e) => {
            let error_msg = format!("Failed to serialize dynamic message: {}", e);
            error!("{}", error_msg);
            return Err(error_msg);
        }
    }

    Ok(())
}

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

    // Add the file descriptor set to the pool
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


#[tracing::instrument]
fn serialize_dynamic_message(dynamic_message: &mut DynamicMessage) -> Result<(), String> {
    info!("serialize_dynamic_message");

    let options = SerializeOptions::new().skip_default_fields(false);

    let mut serializer = serde_json::Serializer::new(vec![]);
    dynamic_message
        .serialize_with_options(&mut serializer, &options)
        .map_err(|e| {
            error!("Failed to serialize DynamicMessage back to JSON: {:?}", e);
            format!("Failed to serialize DynamicMessage back to JSON: {:?}", e)
        })?;

    let serialized_json = String::from_utf8(serializer.into_inner()).map_err(|e| {
        error!("Failed to convert serialized data to UTF-8: {:?}", e);
        format!("Failed to convert serialized data to UTF-8: {:?}", e)
    })?;

    debug!("Serialized JSON: {:?}", serialized_json);

    Ok(())
}

#[tracing::instrument]
fn populate_dynamic_message(
    dynamic_message: &mut DynamicMessage,
    message_descriptor: &MessageDescriptor,
    json_value: &JsonValue,
) -> Result<(), String> {
    info!("populate_dynamic_message");

    if let JsonValue::Object(map) = json_value {
        for (field_name, field_value) in map {
            if let Some(field_descriptor) = message_descriptor.get_field_by_name(field_name) {
                match field_descriptor.kind() {
                    Kind::Double | Kind::Float => {
                        if let Some(float_value) = field_value.as_f64() {
                            let value = ProstReflectValue::F64(float_value);
                            if value.is_valid_for_field(&field_descriptor) {
                                dynamic_message.set_field_by_name(field_name, value);
                                debug!("Field '{}' set to F64 with value {}", field_name, float_value);
                            } else {
                                return Err(format!("Field '{}' expects a float or double value", field_name));
                            }
                        } else {
                            return Err(format!("Field '{}' expects a float or double value", field_name));
                        }
                    }
                    Kind::Int32 | Kind::Sint32 | Kind::Sfixed32 => {
                        if let Some(int_value) = field_value.as_i64() {
                            let value = ProstReflectValue::I32(int_value as i32);
                            if value.is_valid_for_field(&field_descriptor) {
                                dynamic_message.set_field_by_name(field_name, value);
                                debug!("Field '{}' set to I32 with value {}", field_name, int_value);
                            } else {
                                return Err(format!("Field '{}' expects an integer value", field_name));
                            }
                        } else {
                            return Err(format!("Field '{}' expects an integer value", field_name));
                        }
                    }
                    Kind::Int64 | Kind::Sint64 | Kind::Sfixed64 => {
                        if let Some(int_value) = field_value.as_i64() {
                            let value = ProstReflectValue::I64(int_value);
                            if value.is_valid_for_field(&field_descriptor) {
                                dynamic_message.set_field_by_name(field_name, value);
                                debug!("Field '{}' set to I64 with value {}", field_name, int_value);
                            } else {
                                return Err(format!("Field '{}' expects a 64-bit integer value", field_name));
                            }
                        } else {
                            return Err(format!("Field '{}' expects a 64-bit integer value", field_name));
                        }
                    }
                    Kind::Uint32 | Kind::Fixed32 => {
                        if let Some(int_value) = field_value.as_u64() {
                            let value = ProstReflectValue::U32(int_value as u32);
                            if value.is_valid_for_field(&field_descriptor) {
                                dynamic_message.set_field_by_name(field_name, value);
                                debug!("Field '{}' set to U32 with value {}", field_name, int_value);
                            } else {
                                return Err(format!("Field '{}' expects an unsigned integer value", field_name));
                            }
                        } else {
                            return Err(format!("Field '{}' expects an unsigned integer value", field_name));
                        }
                    }
                    Kind::Uint64 | Kind::Fixed64 => {
                        if let Some(int_value) = field_value.as_u64() {
                            let value = ProstReflectValue::U64(int_value);
                            if value.is_valid_for_field(&field_descriptor) {
                                dynamic_message.set_field_by_name(field_name, value);
                                debug!("Field '{}' set to U64 with value {}", field_name, int_value);
                            } else {
                                return Err(format!("Field '{}' expects an unsigned 64-bit integer value", field_name));
                            }
                        } else {
                            return Err(format!("Field '{}' expects an unsigned 64-bit integer value", field_name));
                        }
                    }
                    Kind::Bool => {
                        if let Some(bool_value) = field_value.as_bool() {
                            let value = ProstReflectValue::Bool(bool_value);
                            if value.is_valid_for_field(&field_descriptor) {
                                dynamic_message.set_field_by_name(field_name, value);
                                debug!("Field '{}' set to Bool with value {}", field_name, bool_value);
                            } else {
                                return Err(format!("Field '{}' expects a boolean value", field_name));
                            }
                        } else {
                            return Err(format!("Field '{}' expects a boolean value", field_name));
                        }
                    }
                    Kind::String => {
                        if let Some(string_value) = field_value.as_str() {
                            let value = ProstReflectValue::String(string_value.to_string());
                            if value.is_valid_for_field(&field_descriptor) {
                                dynamic_message.set_field_by_name(field_name, value);
                                debug!("Field '{}' set to String with value {}", field_name, string_value);
                            } else {
                                return Err(format!("Field '{}' expects a string value", field_name));
                            }
                        } else {
                            return Err(format!("Field '{}' expects a string value", field_name));
                        }
                    }
                    Kind::Bytes => {
                        if let Some(string_value) = field_value.as_str() {
                            let bytes = string_value.as_bytes().to_vec();
                            let bytes_for_dyn_message = bytes.clone();
                            let value = ProstReflectValue::Bytes(bytes.into());
                            if value.is_valid_for_field(&field_descriptor) {
                                dynamic_message.set_field_by_name(field_name, value);
                                debug!("Field '{}' set to Bytes with value {:?}", field_name, bytes_for_dyn_message);
                            } else {
                                return Err(format!("Field '{}' expects a byte array value", field_name));
                            }
                        } else {
                            return Err(format!("Field '{}' expects a byte array value", field_name));
                        }
                    }
                    Kind::Enum(enum_descriptor) => {
                        if let Some(enum_value) = field_value.as_str() {
                            if let Some(enum_value) = enum_descriptor.get_value_by_name(enum_value) {
                                let value = ProstReflectValue::EnumNumber(enum_value.number());
                                if value.is_valid_for_field(&field_descriptor) {
                                    dynamic_message.set_field_by_name(field_name, value);
                                    debug!("Field '{}' set to EnumNumber with value {}", field_name, enum_value.number());
                                } else {
                                    return Err(format!("Field '{}' expects a valid enum value", field_name));
                                }
                            } else {
                                return Err(format!("Invalid enum value '{}' for field '{}'", enum_value, field_name));
                            }
                        } else {
                            return Err(format!("Field '{}' expects a valid enum value as a string", field_name));
                        }
                    }
                    Kind::Message(sub_message_descriptor) => {
                        if let Some(nested_value) = field_value.as_object() {
                            let mut nested_message = DynamicMessage::new(sub_message_descriptor.clone());
                            populate_dynamic_message(&mut nested_message, &sub_message_descriptor, field_value)?;
                            let value = ProstReflectValue::Message(nested_message);
                            if value.is_valid_for_field(&field_descriptor) {
                                dynamic_message.set_field_by_name(field_name, value);
                                debug!("Field '{}' set to nested message", field_name);
                            } else {
                                return Err(format!("Field '{}' expects a nested message object", field_name));
                            }
                        } else {
                            return Err(format!("Field '{}' expects a nested message object", field_name));
                        }
                    }
                }
            } else {
                return Err(format!("Field '{}' not found in descriptor", field_name));
            }
        }
    } else {
        return Err("Expected a JSON object to populate DynamicMessage".to_string());
    }

    Ok(())
}