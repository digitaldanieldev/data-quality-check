use anyhow::{anyhow, Context, Result};
use axum::{extract::Json, http::StatusCode, response::IntoResponse};
use serde_json::json;

use thiserror::Error;
use tracing::{debug, error, info, span, Level};

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Failed to load descriptor")]
    LoadDescriptorError(#[source] std::io::Error),

    #[error("Failed to parse JSON: {0}")]
    JsonParseError(#[source] serde_json::Error),

    #[error("Failed to unescape JSON: {0}")]
    JsonUnescapeError(String),

    #[error("Missing environment variable: {0}")]
    MissingEnvVarError(String),

    #[error("Unknown error occurred: {0}")]
    UnknownError(String),
}

impl AppError {
    pub fn to_status_code(&self) -> StatusCode {
        match self {
            AppError::JsonUnescapeError(_) => StatusCode::BAD_REQUEST,
            AppError::JsonParseError(_) => StatusCode::BAD_REQUEST,
            AppError::LoadDescriptorError(_) => StatusCode::INTERNAL_SERVER_ERROR,
            AppError::MissingEnvVarError(_) => StatusCode::BAD_REQUEST,
            AppError::UnknownError(_) => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let status_code = self.to_status_code();
        let body = Json(json!({ "error": self.to_string() }));
        (status_code, body).into_response()
    }
}
