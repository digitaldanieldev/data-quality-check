/* Licensed under the AGPL-3.0 License: https://www.gnu.org/licenses/agpl-3.0.html */

use opentelemetry::{global, metrics::Meter, KeyValue};
use opentelemetry_sdk::{
    metrics::{PeriodicReader, SdkMeterProvider},
    runtime::Tokio,
    Resource,
};
use tracing::{debug, error, info, instrument, trace};

pub fn create_metrics(
    meter: &Meter,
) -> (
    opentelemetry::metrics::Counter<u64>,
    opentelemetry::metrics::Histogram<f64>,
) {
    let request_counter = meter
        .u64_counter("validate_json_requests_total")
        .with_description("Counts the total number of JSON validation requests")
        .build();

    let duration_histogram = meter
        .f64_histogram("validate_json_duration_seconds")
        .with_description("Tracks the duration of JSON validation in seconds")
        .build();

    info!("Created metrics: request_counter and duration_histogram");
    debug!("Counter 'validate_json_requests_total' and histogram 'validate_json_duration_seconds' have been initialized");

    (request_counter, duration_histogram)
}

#[instrument(level = "info")]
pub fn init_meter_provider() -> SdkMeterProvider {
    trace!("Creating metric exporter...");
    let exporter = opentelemetry_stdout::MetricExporterBuilder::default().build();
    info!("Metric exporter created successfully");

    trace!("Building periodic reader with Tokio runtime...");
    let reader = PeriodicReader::builder(exporter, Tokio).build();
    info!("Periodic reader initialized");

    trace!("Creating resource with service name...");
    let resource = Resource::new(vec![KeyValue::new(
        "service.name",
        "json-validation-service",
    )]);
    info!("Resource created with service name: json-validation-service");

    trace!("Building SDK meter provider...");
    let provider = SdkMeterProvider::builder()
        .with_reader(reader)
        .with_resource(resource)
        .build();
    info!("SDK meter provider built successfully");

    trace!("Setting global meter provider...");
    global::set_meter_provider(provider.clone());
    info!("Global meter provider set");

    provider
}
