use futures::future::join_all;
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio;
use tracing::{debug, error, info, span, Level};
use clap::Parser;

use data_quality_settings::{load_env_variables, load_logging_config, parse_log_level};

#[derive(Parser, Debug)]
#[command(version, about = "Load Test", long_about = None)]
struct Args {
    /// Logging level
    #[clap(short, long, default_value = "info")]
    log_level: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct ValidationRequest {
    #[serde(rename = "protobuf")]
    protobuf: String,
    #[serde(rename = "json")]
    json_data: Value,
    #[serde(rename = "json_escaped")]
    json_escaped: bool,
    #[serde(rename = "field_check")]
    field_check: bool,
    #[serde(rename = "field_name")]
    field_name: String,
    #[serde(rename = "field_value_check")]
    field_value_check: i64,
}

impl ValidationRequest {
    fn new_with_field_check(json_data: Value, field_name: String, field_value_check: i64) -> Self {
        Self {
            protobuf: "MyMessage".to_string(),
            json_data,
            json_escaped: false,
            field_check: true,
            field_name,
            field_value_check,
        }
    }

    fn new_without_field_check(json_data: Value) -> Self {
        Self {
            protobuf: "MyMessage".to_string(),
            json_data,
            json_escaped: false,
            field_check: false,
            field_name: "".to_string(),
            field_value_check: 0,
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let cli_args = Args::parse();
    let log_level = parse_log_level(&cli_args.log_level)?;
    let _ = load_logging_config(log_level);
    load_env_variables();

    let span = span!(Level::INFO, "load tester");
    let _enter = span.enter();

    let server_ip = dotenvy::var("DATA_QUALITY_SERVER_IP_TARGET")?;
    let server_port = dotenvy::var("DATA_QUALITY_SERVER_PORT")?;
    let server_address = format!("{}:{}", server_ip, server_port);
    
    // Create a single client instance
    let client = Arc::new(reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()?);

    let sample_data = json!({
        "key1": "example_value",
        "key2": 42,
        "key3": true
    });

    // Create validation requests
    let num_requests = 1000;
    let requests: Vec<_> = (0..num_requests)
        .map(|_| ValidationRequest::new_without_field_check(sample_data.clone()))
        .collect();

    // Measure execution time
    let start_time = Instant::now();
    
    // Send concurrent requests using join_all
    let responses: Vec<Result<reqwest::Response, reqwest::Error>> = join_all(requests.iter().map(|request| {
        let client = Arc::clone(&client);
        let target_url = format!("http://{}/validate", server_address);
        
        async move {
            let res = client.post(&target_url)
                .json(request)
                .send()
                .await;
            res
        }
    })).await;

    // Process results
    let duration = start_time.elapsed();
    let success_count = responses.iter().filter(|r| r.as_ref().unwrap().status().is_success()).count();
    
    info!(
        "Load test completed - Total requests: {}, Successes: {}, Duration: {:?}",
        num_requests,
        success_count,
        duration
    );

    Ok(())
}