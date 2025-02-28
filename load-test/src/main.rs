use futures::future::join_all;
use log::{debug, error, info};
use reqwest;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio; // Import log macros for logging

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

    async fn send(&self, counter: &AtomicUsize) -> Result<Value, Box<dyn std::error::Error>> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build()?;

        debug!("Sending request to validate data: {:?}", self.json_data);
        let response = client
            .post("http://192.168.5.246:8081/validate")
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
    env_logger::init();

    let sample_data = json!({
        "key1": "example_value",
        "key2": 42,
        "key3": true
    });

    let requests: Vec<ValidationRequest> = (0..1000)
        .map(|i| {
            ValidationRequest::new_with_field_check(sample_data.clone(), "key2".to_string(), 42)
        })
        .collect();

    const MAX_CONCURRENCY: usize = 2;
    let total_requests = requests.len();
    let start_time = Instant::now();
    let counter = Arc::new(AtomicUsize::new(0));

    info!("Starting validation with {} requests...", total_requests);
    info!("Concurrency level: {}\n", MAX_CONCURRENCY);

    let mut requests = requests;

    while !requests.is_empty() {
        let chunk_size = std::cmp::min(MAX_CONCURRENCY, requests.len());
        let (chunk, remaining) = requests.split_at(chunk_size);
        let batch_start = Instant::now();

        debug!("Processing a batch of {} requests...", chunk_size);

        let results = join_all(chunk.iter().map(|req| {
            let counter_clone = Arc::clone(&counter);
            async move {
                match req.send(&counter_clone).await {
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
        info!("{:?}", batch_duration);
        let elapsed_time = start_time.elapsed();
        let completed = counter.load(Ordering::Relaxed);
        let rps = if elapsed_time.as_secs() > 0 {
            (completed as f64) / elapsed_time.as_secs_f64()
        } else {
            0.0
        };

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
