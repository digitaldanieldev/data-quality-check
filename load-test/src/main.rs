/* Licensed under the AGPL-3.0 License: https://www.gnu.org/licenses/agpl-3.0.html */

use clap::Parser;
use futures::future::join_all;
use reqwest::{self, Client};
use serde::{Deserialize, Serialize};
use serde_json;
use serde_json::{json, Value};
use std::fs::File;
use std::io::Read;
use std::io::{self, Write};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio;
use tokio::sync::Semaphore;
use tokio::time::sleep;
use tracing::{debug, error, info, span, trace, warn, Level};

use data_quality_settings::{load_env_variables, load_logging_config, parse_log_level};

#[derive(Clone, Parser, Debug)]
#[command(version, about = "Load Test", long_about = None)]
struct Args {
    /// Logging level
    #[clap(short, long, default_value = "info")]
    log_level: String,

    /// Number of times to run the load test
    #[clap(short, long, default_value_t = 1)]
    iterations: u32,

    /// Number of concurrent requests (semaphore permits)
    #[clap(short, long, default_value_t = 100)]
    semaphore_permits: usize,

    /// Number of validation requests to send
    #[clap(short, long, default_value_t = 2000)]
    num_requests: usize,

    /// Maximum idle connections per host
    #[clap(short, long, default_value_t = 100)]
    pool_max_idle_per_host: usize,

    /// Timeout for the client in seconds
    #[clap(short, long, default_value_t = 100)]
    timeout_secs: u64,

    /// Option to generate load test configurations with custom parameters
    #[clap(long, action = clap::ArgAction::SetTrue, default_value_t = false)]
    generate_config: bool,

    /// Range for semaphore_permits (start,end,step)
    #[clap(long, value_parser = parse_range)]
    semaphore_permits_range: Option<(usize, usize, usize)>,

    /// Range for pool_max_idle_per_host (start,end,step)
    #[clap(long, value_parser = parse_range)]
    pool_max_idle_per_host_range: Option<(usize, usize, usize)>,

    /// Range for num_requests (start,end,step)
    #[clap(long, value_parser = parse_range)]
    num_requests_range: Option<(usize, usize, usize)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct LoadTestConfig {
    semaphore_permits: usize,
    num_requests: usize,
    pool_max_idle_per_host: usize,
    timeout_secs: u64,
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

    if cli_args.generate_config {
        info!("Generating load test configurations.");
        let configs = generate_configs(
            cli_args.semaphore_permits_range,
            cli_args.pool_max_idle_per_host_range,
            cli_args.num_requests_range,
        );

        let file_path = "load_test_configs.json";
        save_configs_to_file(configs, file_path)?;

        println!("Configurations saved to {}", file_path);
    } else {
        info!("Loading load test configurations from file.");
        let file_path = "load_test_configs.json";
        let mut file = File::open(file_path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;

        let test_configs: Vec<LoadTestConfig> = serde_json::from_str(&contents)?;

        for config in test_configs {
            info!("Running load test with config: {:?}", config);
            let (total_requests, total_duration) = run_load_test(cli_args.clone(), &config).await?;
            info!(
                "Completed load test - Total requests: {}, Duration: {:?}",
                total_requests, total_duration
            );

            info!("");
        }
    }

    Ok(())
}

fn generate_configs(
    semaphore_range: Option<(usize, usize, usize)>,
    pool_range: Option<(usize, usize, usize)>,
    num_requests_range: Option<(usize, usize, usize)>,
) -> Vec<LoadTestConfig> {
    trace!("Generating load test configurations.");

    let mut configs = Vec::new();

    let (semaphore_start, semaphore_end, semaphore_step) = semaphore_range.unwrap_or((10, 200, 10));
    let (pool_start, pool_end, pool_step) = pool_range.unwrap_or((10, 100, 10));
    let (num_requests_start, num_requests_end, num_requests_step) =
        num_requests_range.unwrap_or((500, 5000, 100));

    for semaphore_permits in (semaphore_start..=semaphore_end).step_by(semaphore_step) {
        for pool_max_idle_per_host in (pool_start..=pool_end).step_by(pool_step) {
            for num_requests in (num_requests_start..=num_requests_end).step_by(num_requests_step) {
                configs.push(LoadTestConfig {
                    semaphore_permits,
                    num_requests,
                    pool_max_idle_per_host,
                    timeout_secs: 60,
                });
            }
        }
    }

    trace!("Generated {} configurations.", configs.len());

    configs
}

fn parse_range(input: &str) -> Result<(usize, usize, usize), String> {
    trace!("Parsing range from input: {}", input);

    let parts: Vec<&str> = input.split(',').collect();
    if parts.len() != 3 {
        error!("Invalid range format: {}", input);
        return Err(format!("Invalid range format: {}", input));
    }

    let start: usize = parts[0]
        .parse()
        .map_err(|_| format!("Invalid start value: {}", parts[0]))?;
    let end: usize = parts[1]
        .parse()
        .map_err(|_| format!("Invalid end value: {}", parts[1]))?;
    let step: usize = parts[2]
        .parse()
        .map_err(|_| format!("Invalid step value: {}", parts[2]))?;

    trace!("Parsed range: start={}, end={}, step={}", start, end, step);

    Ok((start, end, step))
}

async fn run_load_test(
    cli_args: Args,
    config: &LoadTestConfig,
) -> Result<(usize, Duration), Box<dyn std::error::Error + Send + Sync>> {
    trace!("Starting load test with configuration: {:?}", config);

    let log_level = parse_log_level(&cli_args.log_level)?;
    let _ = load_logging_config(log_level);
    load_env_variables();

    let server_ip = dotenvy::var("DATA_QUALITY_SERVER_IP_HOST")?;
    let server_port = dotenvy::var("DATA_QUALITY_SERVER_PORT")?;
    let server_address = format!("{}:{}", server_ip, server_port);

    let client = Arc::new(
        reqwest::Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .pool_max_idle_per_host(config.pool_max_idle_per_host)
            .build()?,
    );

    let sample_data = json!({
        "key1": "example_value",
        "key2": 42,
        "key3": true
    });

    let requests: Vec<_> = (0..config.num_requests)
        .map(|_| ValidationRequest::new_without_field_check(sample_data.clone()))
        .collect();

    let mut total_requests = 0;
    let mut total_duration = Duration::new(0, 0);

    let buffer_factor = 200;
    let total_file_descriptors = config.num_requests * cli_args.iterations as usize + buffer_factor;

    info!(
        "Calculated file descriptors (including buffer): {}. You may want to adjust your ulimit to this value.",
        total_file_descriptors
    );

    let semaphore = Arc::new(Semaphore::new(config.semaphore_permits));

    let mut tasks = Vec::new();

    for i in 1..=cli_args.iterations {
        let iteration_span =
            span!(Level::INFO, "load_test_iteration", iteration = i, config = ?config);
        let _enter = iteration_span.enter();

        let client = Arc::clone(&client);
        let server_address = server_address.clone();
        let requests = requests.clone();
        let semaphore = Arc::clone(&semaphore);

        let task = tokio::spawn(async move {
            info!("Starting iteration {}", i);

            let start_time = Instant::now();

            let responses: Vec<Result<reqwest::Response, reqwest::Error>> =
                join_all(requests.iter().map(|request| {
                    let client = Arc::clone(&client);
                    let target_url = format!("http://{}/validate", server_address);
                    let permit = Arc::clone(&semaphore);

                    async move {
                        let _permit = permit.acquire().await.unwrap();
                        send_request_with_retry(&client, &target_url, request).await
                    }
                }))
                .await;

            let duration = start_time.elapsed();
            let success_count = responses
                .iter()
                .filter(|r| r.as_ref().unwrap().status().is_success())
                .count();

            info!(
                "Iteration {} completed - Total requests: {}, Successes: {}, Duration: {:?}",
                i,
                requests.len(),
                success_count,
                duration
            );

            info!("");

            (requests.len(), duration)
        });

        tasks.push(task);
    }

    let results: Vec<(usize, Duration)> = join_all(tasks)
        .await
        .into_iter()
        .filter_map(|result| result.ok())
        .collect();

    for (iteration_requests, iteration_duration) in results {
        total_requests += iteration_requests;
        total_duration += iteration_duration;
    }

    let avg_duration = if cli_args.iterations > 0 {
        total_duration / cli_args.iterations
    } else {
        Duration::new(0, 0)
    };

    let avg_duration = avg_duration.as_secs_f64();
    let rps: f64 = if avg_duration > 0.0 {
        total_requests as f64 / avg_duration
    } else {
        0.0
    };

    info!(
        "Load test completed for all iterations - Total requests: {}, Average duration: {:?}, Requests per second: {:.2}",
        total_requests,
        avg_duration,
        rps
    );

    info!("");

    Ok((total_requests, total_duration))
}

fn save_configs_to_file(configs: Vec<LoadTestConfig>, file_path: &str) -> io::Result<()> {
    trace!("Saving configurations to file: {}", file_path);

    let mut file = File::create(file_path)?;
    let json_str = serde_json::to_string_pretty(&configs)?;

    writeln!(file, "{}", json_str)?;

    info!("Configurations saved successfully to {}", file_path);

    Ok(())
}

async fn send_request_with_retry(
    client: &Client,
    url: &str,
    request: &ValidationRequest,
) -> Result<reqwest::Response, reqwest::Error> {
    let mut retries = 0;
    let max_retries = 5;
    let backoff_duration = Duration::from_secs(1);

    loop {
        trace!("Sending request to {}", url);

        let response = client.post(url).json(request).send().await;

        match response {
            Ok(res) => {
                info!("Request succeeded.");
                return Ok(res);
            }
            Err(e) if retries < max_retries => {
                retries += 1;
                let backoff_time = backoff_duration * 2_u32.pow(retries);
                warn!(
                    "Request failed, retrying in {:?}. Error: {}",
                    backoff_time, e
                );
                sleep(backoff_time).await;
            }
            Err(e) => {
                error!("Request failed after {} retries. Error: {}", retries, e);
                return Err(e);
            }
        }
    }
}
