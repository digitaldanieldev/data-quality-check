use futures::future::join_all;
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::{Arc, atomic::{AtomicUsize, Ordering}};
use std::time::{Duration, Instant};
use tokio;
use tracing::{info, span, Level};
use clap::Parser;

use data_quality_settings::{load_env_variables, load_logging_config, parse_log_level};

#[derive(Parser, Debug)]
#[command(version, about = "Load Test", long_about = None)]
struct Args {
    /// Logging level
    #[clap(short, long, default_value = "info")]
    log_level: String,

    /// Number of times to run the load test
    #[clap(short, long, default_value_t = 1)]
    iterations: u32,
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

    let server_ip = dotenvy::var("DATA_QUALITY_SERVER_IP_HOST")?;
    let server_port = dotenvy::var("DATA_QUALITY_SERVER_PORT")?;
    let server_address = format!("{}:{}", server_ip, server_port);
    
    let client = Arc::new(reqwest::Client::builder()
    .timeout(Duration::from_secs(100))
    .pool_max_idle_per_host(100) 
    .build()?);

    let sample_data = json!({
        "key1": "example_value",
        "key2": 42,
        "key3": true
    });

    // Create validation requests
    let num_requests = 4000;
    let requests: Vec<_> = (0..num_requests)
        .map(|_| ValidationRequest::new_without_field_check(sample_data.clone()))
        .collect();

    // Variables to accumulate total requests and total duration
    let mut total_requests = 0;
    let mut total_duration = Duration::new(0, 0);

    // Buffer and calculation for file descriptors
    let buffer_factor = 200;  // This buffer factor allows for some extra overhead
    let total_file_descriptors = num_requests * cli_args.iterations as usize + buffer_factor;

    info!(
        "Calculated file descriptors (including buffer): {}. You may want to adjust your ulimit to this value.",
        total_file_descriptors
    );

    // Create a vector to hold all the spawned tasks for each iteration
    let mut tasks = Vec::new();

    // Spawn tasks for each iteration to run concurrently
    for i in 1..=cli_args.iterations {
        let client = Arc::clone(&client);
        let server_address = server_address.clone();
        let requests = requests.clone();

        // Spawn a new task for each iteration
        let task = tokio::spawn(async move {
            info!("Starting iteration {}", i);

            // Measure execution time for this iteration
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

            // Process results for this iteration
            let duration = start_time.elapsed();
            let success_count = responses.iter().filter(|r| r.as_ref().unwrap().status().is_success()).count();

            info!(
                "Iteration {} completed - Total requests: {}, Successes: {}, Duration: {:?}",
                i,
                requests.len(),
                success_count,
                duration
            );

            // Return the number of requests and the duration for this iteration
            (requests.len(), duration)
        });

        // Add the task to the vector
        tasks.push(task);
    }

    // Wait for all tasks to complete and accumulate the results
    let results: Vec<(usize, Duration)> = join_all(tasks).await
        .into_iter()
        // Unwrap the results to only get successful ones, ignore errors
        .filter_map(|result| result.ok()) 
        .collect();

    // Calculate total requests and total duration
    for (iteration_requests, iteration_duration) in results {
        total_requests += iteration_requests;
        total_duration += iteration_duration;
    }

    // Calculate average duration
    let avg_duration = if cli_args.iterations > 0 {
        total_duration / cli_args.iterations
    } else {
        Duration::new(0, 0)
    };

    // Calculate requests per second (RPS)
    let avg_duration = avg_duration.as_secs_f64();
    let rps: f64 = if avg_duration > 0.0 {
        total_requests as f64 / avg_duration
    } else {
        0.0
    };

    // Output the total number of requests, average duration, and requests per second
    info!(
        "Load test completed for all iterations - Total requests: {}, Average duration: {:?}, Requests per second: {:.2}",
        total_requests,
        avg_duration,
        rps
    );

    Ok(())
}
