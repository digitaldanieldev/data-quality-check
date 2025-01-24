use anyhow::Result;
use base64;
use clap::Parser;
use dotenvy;
use prost::Message;
use prost_types::FileDescriptorSet;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    env,
    error::Error,
    ffi::OsStr,
    path::{Path, PathBuf},
    process::Command as StdCommand, sync::Arc,
};
use tokio::fs;
use tokio::sync::Mutex as TokioMutex;
use tokio::time::{sleep, Duration};
use tracing::{error, info, span, Level};
use walkdir::WalkDir;

use data_quality_settings::{load_env_variables, load_logging_config};

#[derive(Parser, Debug)]
#[command(version, about = "Proto Producer", long_about = None)]
struct Args {
    /// Run the program in a loop
    #[arg(long, action(clap::ArgAction::SetTrue))]
    loop_mode: bool,

    /// Interval in seconds for each iteration if loop mode is enabled
    #[arg(long, default_value = "10", value_parser = clap::value_parser!(u64))]
    interval: u64,

    /// Logging level
    #[clap(short, long, default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli_args = Args::parse();

    // If `log_level` argument is provided, set level
    let log_level = match cli_args.log_level.to_lowercase().as_str() {
        "error" => Level::ERROR,
        "warn" => Level::WARN,
        "info" => Level::INFO,
        "debug" => Level::DEBUG,
        "trace" => Level::TRACE,
        _ => Level::INFO,
    };

    loop {
        
        let _ = load_logging_config(log_level);
        load_env_variables();

        let span = span!(Level::INFO, "proto producer");
        let _enter = span.enter();

        let proto_schema_input_dir = dotenvy::var("PROTO_SCHEMA_INPUT_DIR")?;
        let server_ip = dotenvy::var("SERVER_IP")?;
        let server_port = dotenvy::var("SERVER_PORT")?;
        let server_address = format!("{}:{}", server_ip, server_port);

        let protobuf_definitions: Arc<TokioMutex<HashMap<String, (Vec<u8>, u64)>>> =
            Arc::new(TokioMutex::new(HashMap::new()));
        let file_timestamps: Arc<TokioMutex<HashMap<String, u64>>> =
            Arc::new(TokioMutex::new(HashMap::new()));

        {
            let mut definitions = protobuf_definitions.lock().await;
            let mut timestamps = file_timestamps.lock().await;

            match load_proto_files(&proto_schema_input_dir, &mut definitions, &mut timestamps).await {
                Ok(_) => info!("Successfully loaded proto files."),
                Err(err) => {
                    eprintln!("Error loading proto files: {}", err);
                    continue;
                }
            }
        }

        for (file_name, (file_content, _)) in &*protobuf_definitions.lock().await {
            info!(
                "Serializing and sending FileDescriptorSet for: {}",
                file_name
            );

            let fd_set_result = prost::Message::decode(file_content.as_slice());
            let fd_set = match fd_set_result {
                Ok(fd_set) => fd_set,
                Err(err) => {
                    eprintln!("Failed to decode FileDescriptorSet for {}: {}", file_name, err);
                    continue; 
                }
            };

            let serialized_fd_set = serialize_file_descriptor_set(&fd_set);
            let descriptor_server_url = format!("http://{}/load_descriptor", server_address);
            info!("descriptor_server_url: {}", &descriptor_server_url);

            match send_to_axum_server(&descriptor_server_url, &file_name, &serialized_fd_set).await {
                Ok(success_message) => info!("{}", success_message),
                Err(err) => eprintln!("Error sending FileDescriptorSet for {}: {}", file_name, err),
            }
        }

        if !cli_args.loop_mode {
            break;
        }

        sleep(Duration::from_secs(cli_args.interval)).await;
    }

    Ok(())
}


#[tracing::instrument]
fn serialize_file_descriptor_set(fd_set: &FileDescriptorSet) -> Vec<u8> {
    info!("serialize_file_descriptor_set");

    let mut buf = Vec::new();
    fd_set
        .encode(&mut buf)
        .expect("Failed to encode FileDescriptorSet");
    buf
}

#[derive(Serialize, Deserialize)]
struct LoadDescriptorRequest {
    file_name: String,
    file_content: String,
}

async fn send_to_axum_server(
    url: &str,
    file_name: &str,
    data: &[u8],
) -> Result<String, Box<dyn Error>> {
    let span = span!(Level::INFO, "send_to_axum_server");
    let _enter = span.enter();

    let client = Client::new();

    let payload = LoadDescriptorRequest {
        file_name: file_name.to_string(),
        file_content: base64::encode(data),
    };

    let response = client
        .post(url)
        .json(&payload) 
        .send()
        .await?;

    if response.status() == StatusCode::OK {
        let success_message = format!("Successfully sent FileDescriptorSet for: {}", file_name);
        info!("{}", success_message);
        Ok(success_message)
    } else {
        let error_message = format!(
            "Failed to send FileDescriptorSet for {}: {:?}",
            file_name,
            response.status()
        );
        error!("{}", error_message);
        Err(Box::new(std::io::Error::new(std::io::ErrorKind::Other, error_message)))
    }
}

async fn load_proto_files(
    proto_schema_input_dir: &str,
    definitions: &mut HashMap<String, (Vec<u8>, u64)>,
    file_timestamps: &mut HashMap<String, u64>,
) -> Result<(), Box<dyn std::error::Error>> {
    let span = span!(Level::INFO, "load_proto_files");
    let _enter = span.enter();

    let proto_schema_input_dir = resolve_relative_path(proto_schema_input_dir)?;
    let proto_output_dir = resolve_relative_path(&env::var("PROTO_SCHEMA_GENPB_DIR")?)?;
    fs::create_dir_all(&proto_output_dir).await?;

    let protoc_path = env::var("PROTOC_PATH")?;
    let mut updated_files = Vec::new();

    for file in proto_files_in_directory(&proto_schema_input_dir)? {
        let metadata = file.metadata()?;
        let modified_time = metadata
            .modified()?
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs();

        let file_name = file.file_name().unwrap().to_string_lossy().into_owned();

        if file_timestamps.get(&file_name).copied() != Some(modified_time) {
            info!("File changed or new: {:?}", file);

            if let Err(e) = process_proto_file(
                &file,
                &proto_output_dir,
                &protoc_path,
                definitions,
                &mut updated_files,
            )
            .await
            {
                error!("Error processing file {:?}: {}", file, e);
            } else {
                file_timestamps.insert(file_name, modified_time);
            }
        }
    }

    log_updated_files(&updated_files, definitions);
    Ok(())
}

#[tracing::instrument]
fn resolve_relative_path(path: &str) -> Result<PathBuf, Box<dyn std::error::Error>> {
    info!("resolve_relative_path");

    let current_dir = env::current_dir()?;
    Ok(if Path::new(path).is_absolute() {
        PathBuf::from(path)
    } else {
        current_dir.join(path)
    })
}

#[tracing::instrument]
fn proto_files_in_directory(
    proto_schema_input_dir: &Path,
) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
    info!("proto_files_in_directory");

    WalkDir::new(proto_schema_input_dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.path().extension() == Some(OsStr::new("proto")))
        .map(|entry| Ok(entry.path().to_path_buf()))
        .collect()
}

async fn process_proto_file(
    file: &Path,
    proto_output_dir: &Path,
    protoc_path: &str,
    definitions: &mut HashMap<String, (Vec<u8>, u64)>,
    updated_files: &mut Vec<String>,
) -> Result<(), Box<dyn std::error::Error>> {
    let span = span!(Level::INFO, "process_proto_file");
    let _enter = span.enter();

    let output_file = generate_output_file(file, proto_output_dir);
    compile_with_protoc(protoc_path, file, &output_file).await?;

    let file_content = fs::read(&output_file).await?;

    let file_name = file.file_name().unwrap().to_string_lossy().into_owned();

    let modified_time = get_modified_time(file).await?;

    if definitions
        .entry(file_name.clone())
        .or_insert((file_content.clone(), modified_time))
        .1
        != modified_time
    {
        info!("File changed: {}", file_name);
        updated_files.push(file_name.clone());
    }

    Ok(())
}

async fn get_modified_time(file: &Path) -> Result<u64, Box<dyn std::error::Error>> {
    let span = span!(Level::INFO, "get_modified_time");
    let _enter = span.enter();

    let metadata = fs::metadata(file).await?;
    let modified_time = metadata
        .modified()?
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();
    Ok(modified_time)
}

#[tracing::instrument]
fn generate_output_file(file: &Path, proto_output_dir: &Path) -> PathBuf {
    info!("generate_output_file");

    proto_output_dir.join(format!(
        "{}.pb",
        file.file_stem().unwrap().to_str().unwrap()
    ))
}

async fn compile_with_protoc(
    protoc_path: &str,
    file: &Path,
    output_file: &Path,
) -> Result<(), Box<dyn std::error::Error>> {
    let span = span!(Level::INFO, "compile_with_protoc");
    let _enter = span.enter();

    let status = StdCommand::new(protoc_path)
        .arg("--proto_path")
        .arg(file.parent().unwrap().to_str().unwrap())
        .arg("--descriptor_set_out")
        .arg(output_file)
        .arg(file)
        .status()?;

    if !status.success() {
        return Err("Protoc compilation failed".into());
    }

    Ok(())
}

#[tracing::instrument]
fn log_updated_files(updated_files: &[String], definitions: &HashMap<String, (Vec<u8>, u64)>) {
    if !updated_files.is_empty() {
        info!("Updated files: {:?}", updated_files);
    }
    for (file_name, (_, timestamp)) in definitions.iter() {
        info!("Loaded definition: {}, timestamp: {}", file_name, timestamp);
    }
}