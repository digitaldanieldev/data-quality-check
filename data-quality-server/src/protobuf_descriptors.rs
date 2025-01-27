use anyhow::Result;
use axum::{
    extract::{Json, State},
    http::StatusCode,
    response::IntoResponse,
};
use base64;
use prost_reflect::DescriptorPool;
use prost_types::FileDescriptorSet;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use tracing::{debug, error, info, span, Level};

use crate::AppState;

type DescriptorMap = Arc<Mutex<HashMap<String, Vec<u8>>>>;

/////////////////////
// load descriptor //
/////////////////////

#[tracing::instrument]
pub fn load_descriptor(
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
pub fn load_descriptors(
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
pub struct LoadDescriptorRequest {
    pub file_name: String,
    pub file_content: String,
}

pub fn rebuild_descriptor_pool(
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
