/* Licensed under the AGPL-3.0 License: https://www.gnu.org/licenses/agpl-3.0.html */

use crate::protobuf_descriptors::rebuild_descriptor_pool;
use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
};
use base64;
use serde::Deserialize;
use serde_json::{json, Value as JsonValue};
use tracing::{debug, error, info, span, Level};

use crate::json_validation::{unescape_json, validate_json};
use crate::protobuf_descriptors::LoadDescriptorRequest;
use crate::AppState;

////////////////////////////////
// descriptor loading handler //
////////////////////////////////

pub async fn load_descriptor_handler(
    State(state): State<AppState>,
    Json(payload): Json<LoadDescriptorRequest>,
) -> impl IntoResponse {
    let permit = match state.semaphore.acquire().await {
        Ok(permit) => permit,
        Err(_) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                "Too many concurrent requests, please try again later".to_string(),
            )
                .into_response();
        }
    };

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

    let mut descriptor_map = state.descriptor_map.write().await;
    descriptor_map.insert(file_name.clone(), file_content.clone());

    info!("Descriptor {} loaded successfully.", file_name);
    (
        StatusCode::OK,
        format!("Descriptor {} loaded successfully.", file_name),
    )
        .into_response()
}

#[derive(Deserialize)]
pub struct ValidationRequest {
    pub protobuf: Option<String>,
    pub json: serde_json::Value,
    pub json_escaped: Option<bool>,
    pub field_check: Option<bool>,
    pub field_name: Option<String>,
    pub field_value_check: Option<serde_json::Value>,
}

pub async fn validate_json_handler(
    State(state): State<AppState>,
    Json(payload): Json<ValidationRequest>,
) -> Result<impl IntoResponse, StatusCode> {
    let permit = match state.semaphore.acquire().await {
        Ok(permit) => permit,
        Err(_) => {
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }
    };

    let span = span!(Level::INFO, "validate_json_handler");
    let _enter = span.enter();

    let proto_name = payload.protobuf;

    let json_escaped = payload.json_escaped.unwrap_or(true);
    let json_message = if json_escaped {
        unescape_json(&payload.json.to_string()).map_err(|e| {
            error!("Failed to unescape JSON: {}", e);
            e.to_status_code()
        })?
    } else {
        payload.json.to_string()
    };

    let descriptor_pool = {
        let descriptor_map = state.descriptor_map.read().await;
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
