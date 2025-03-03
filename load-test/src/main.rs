use clap::Parser;
use data_quality_settings::{load_env_variables, load_logging_config, parse_log_level};
use futures::future::join_all;
use reqwest::{redirect, Client};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::{Duration, Instant};
use tokio;
use tokio::sync::{mpsc, Semaphore};
use tracing::{debug, error, info, span, Level};

// increase the limit temporarily for the current session:
// ulimit -n 100000

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

#[tokio::main(flavor = "multi_thread", worker_threads = 4)]
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

    let client = Arc::new(
        reqwest::Client::builder()
            .timeout(Duration::from_secs(20))
            .pool_max_idle_per_host(100)
            .tcp_keepalive(Some(Duration::from_secs(60)))
            .redirect(redirect::Policy::none())
            .build()?,
    );

    // Prepare test data
    let sample_data = json!({
        "key1": "example_value",
        "key2": 42,
        "key3": true
    });

    // Create validation requests
    let num_requests = 10000;
    let part_num_requests = num_requests / 2;

    let requests_with_field_check: Vec<_> = (0..part_num_requests)
        .map(|_| {
            ValidationRequest::new_with_field_check(
                sample_data.clone(),
                "field_name".to_string(),
                42,
            )
        })
        .collect();

    let requests_without_field_check: Vec<_> = (0..part_num_requests)
        .map(|_| ValidationRequest::new_without_field_check(sample_data.clone()))
        .collect();

    // Combine both sets of requests
    let requests: Vec<_> = requests_with_field_check
        .into_iter()
        .chain(requests_without_field_check.into_iter())
        .collect();

    // Create optimized semaphore and channel
    let semaphore = Arc::new(Semaphore::new(500));
    let (tx, mut rx) = mpsc::channel::<ValidationRequest>(1000); // Increased buffer size

    // Spawn response processor task
    tokio::spawn(async move {
        while let Some(request) = rx.recv().await {
            // debug!("Processing request for field_name: {}", request.field_name);
        }
    });

    // Measure execution time
    let start_time = Instant::now();

    // Execute concurrent requests with optimized parameters
    let responses: Vec<Result<reqwest::Response, reqwest::Error>> =
        join_all(requests.iter().map(|request| {
            let client = Arc::clone(&client);
            let target_url = format!("http://{}/validate", server_address);
            let semaphore_clone = Arc::clone(&semaphore);
            let tx_clone = tx.clone();
            async move {
                let _permit = semaphore_clone.acquire().await;
                let _ = tx_clone.send(request.clone()).await;

                let res = client
                    .post(&target_url)
                    .json(request)
                    //.header("Accept-Encoding", "gzip")
                    .send()
                    .await;
                res
            }
        }))
        .await;

    let duration = start_time.elapsed();
    let success_count = responses
        .iter()
        .filter(|r| r.as_ref().unwrap().status().is_success())
        .count();

    info!(
        "Load test completed - Total requests: {}, Successes: {}, Duration: {:?}",
        num_requests, success_count, duration
    );

    Ok(())
}
