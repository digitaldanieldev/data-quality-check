/* Licensed under the AGPL-3.0 License: https://www.gnu.org/licenses/agpl-3.0.html */

use anyhow::Result;
use dynamic_message::{populate_dynamic_message, serialize_dynamic_message};
use opentelemetry::{global, KeyValue};
use prost_reflect::{DescriptorPool, DynamicMessage};
use serde_json::Value as JsonValue;
use std::time::Instant;
use tracing::{debug, error, info, span, Level, trace, warn};

use crate::app_error::AppError;
use crate::metrics::create_metrics;



#[tracing::instrument]
pub fn validate_json(
    descriptor_pool: Option<&DescriptorPool>,
    json_message: &str,
    definition_name: Option<&str>,
    field_check: Option<bool>,
    field_name: Option<String>,
    field_value_check: Option<JsonValue>,
    enable_metrics: bool,
) -> Result<(), anyhow::Error> {
    info!("Starting JSON validation process.");

    let meter = if enable_metrics {
        Some(global::meter("json-validation-service"))
    } else {
        None
    };

    let start_time = meter.as_ref().map(|_| Instant::now());

    let json_value: JsonValue = serde_json::from_str(json_message).map_err(|e| {
        let error_msg = format!("Failed to parse JSON: {:?}", e);
        error!("{}", error_msg);
        anyhow::anyhow!(error_msg)
    })?;

    let message_name = definition_name.unwrap_or("only_json").to_string();

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

        if field_check.unwrap_or(false) {
            debug!("Performing field check validation.");
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
        info!("No definition_name provided. Only parsed JSON successfully.");

        if field_check.unwrap_or(false) {
            debug!("Performing field check validation on parsed JSON.");
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

    info!("JSON validation completed.");
    Ok(())
}

pub fn unescape_json(json_string: &str) -> Result<String, AppError> {
    trace!("Attempting to unescape JSON string.");
    
    if json_string.starts_with('"') && json_string.ends_with('"') {
        let unescaped = serde_json::from_str::<String>(json_string)
            .map_err(|e| AppError::JsonUnescapeError(format!("Failed to unescape JSON: {}", e)))?;
        info!("Successfully unescaped JSON string.");
        Ok(unescaped)
    } else {
        info!("JSON string does not need unescaping.");
        Ok(json_string.to_string())
    }
}

pub fn validate_json_message_content(
    json_value: &JsonValue,
    field_name: Option<String>,
    field_value_check: Option<JsonValue>,
) -> Result<(), String> {
    trace!("Starting field content validation.");

    if let (Some(field), Some(expected_value)) = (field_name, field_value_check) {
        if let Some(actual_value) = json_value.get(&field) {
            if actual_value != &expected_value {
                let error_msg = format!(
                    "Field '{}' value mismatch: expected {:?}, found {:?}",
                    field, expected_value, actual_value
                );
                error!("{}", error_msg);
                return Err(error_msg);
            }
            info!("Field '{}' value matched expected value.", field);
            Ok(())
        } else {
            let error_msg = format!("Field '{}' not found in the JSON", field);
            error!("{}", error_msg);
            Err(error_msg)
        }
    } else {
        let error_msg = "Field name and value must be provided for validation".to_string();
        error!("{}", error_msg);
        Err(error_msg)
    }
}
