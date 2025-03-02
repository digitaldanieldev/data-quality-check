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

    async fn send(&self, counter: &AtomicUsize, test_target: &String) -> Result<Value, Box<dyn std::error::Error>> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()?;

        debug!("Sending request to validate data: {:?}", self.json_data);
        let response = client
            .post(test_target)
            .header("Content-Type", "application/json")
            .json(self)
            .send()
            .await?;

        let result = response.json::<Value>().await?;

        counter.fetch_add(1, Ordering::Relaxed);
        Ok(result)
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cli_args = Args::parse();
    let log_level = parse_log_level(&cli_args.log_level)?;
    let _ = load_logging_config(log_level);
    load_env_variables();

    let span = span!(Level::INFO, "load tester");
    let _enter = span.enter();

    let server_ip = dotenvy::var("DATA_QUALITY_SERVER_IP_TARGET")?;
    let server_port = dotenvy::var("DATA_QUALITY_SERVER_PORT")?;
    let server_address = format!("{}:{}", server_ip, server_port);
    let load_test_target = Arc::new(format!("http://{}/validate", server_address));

    let sample_data = json!({
        "key1": "example_value",
        "key2": 42,
        "key3": true
    });

    let requests: Vec<ValidationRequest> = (0..100)
        .map(|i| {
            ValidationRequest::new_with_field_check(sample_data.clone(), "key2".to_string(), 42)
        })
        .collect();

    const MAX_CONCURRENCY: usize = 20;
    let total_requests = requests.len();
    let start_time = Instant::now();
    let counter = Arc::new(AtomicUsize::new(0));

    info!("Starting validation with {} requests...", total_requests);
    info!("Concurrency level: {}\n", MAX_CONCURRENCY);

    // Create a single client outside the loop to reuse across all requests
    let client = reqwest::Client::new();

    let mut requests = requests;

    while !requests.is_empty() {
        let chunk_size = std::cmp::min(MAX_CONCURRENCY, requests.len());
        let (chunk, remaining) = requests.split_at(chunk_size);
        let batch_start = Instant::now();

        debug!("Processing a batch of {} requests...", chunk_size);

        let results = join_all(chunk.iter().map(|req| {
            let counter_clone = Arc::clone(&counter);
            let load_test_target_clone = Arc::clone(&load_test_target);  // Works!
            async move {
                match req.send(&counter_clone, &load_test_target_clone).await {
                    Ok(_) => Some(Ok(())),
                    Err(e) => {
                        error!("Error in request: {}", e);
                        Some(Err(e))
                    }
                }
            }
        }))
        .await;

        let batch_completed = results
            .iter()
            .filter(|r| r.as_ref().unwrap().is_ok())
            .count();

        let batch_duration = batch_start.elapsed();
        let elapsed_time = start_time.elapsed();
        let completed = counter.load(Ordering::Relaxed);
        let rps = if elapsed_time.as_secs() > 0 {
            (completed as f64) / elapsed_time.as_secs_f64()
        } else {
            0.0
        };

        // Log the batch statistics
        info!("Batch statistics:");
        info!("  Time taken: {:.2?}", batch_duration);
        info!(
            "  Requests completed in batch: {}/{}",
            batch_completed, chunk_size
        );
        info!(
            "  Total completed: {}/{} ({:.1}%)",
            completed,
            total_requests,
            (completed as f64 / total_requests as f64 * 100.0)
        );
        info!("  Current RPS: {:.1}\n", rps);

        // Move to the next batch
        requests = remaining.to_vec();
    }

    let total_duration = start_time.elapsed();
    let final_rps = if total_duration.as_secs() > 0 {
        (counter.load(Ordering::Relaxed) as f64) / total_duration.as_secs_f64()
    } else {
        0.0
    };

    info!("Final Statistics:");
    info!("-------------");
    info!("Total duration: {:#?}", total_duration);
    info!(
        "Total requests completed: {}/{}",
        counter.load(Ordering::Relaxed),
        total_requests
    );
    info!("Average requests per second: {:.1}", final_rps);

    Ok(())
}
