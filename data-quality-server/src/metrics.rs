/* Licensed under the AGPL-3.0 License: https://www.gnu.org/licenses/agpl-3.0.html */

use opentelemetry::{global, metrics::Meter, KeyValue};
use opentelemetry_sdk::{
    metrics::{PeriodicReader, SdkMeterProvider},
    runtime::Tokio,
    Resource,
};

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

    (request_counter, duration_histogram)
}

pub fn init_meter_provider() -> SdkMeterProvider {
    let exporter = opentelemetry_stdout::MetricExporterBuilder::default().build();
    let reader = PeriodicReader::builder(exporter, Tokio).build();
    let resource = Resource::new(vec![KeyValue::new(
        "service.name",
        "json-validation-service",
    )]);
    let provider = SdkMeterProvider::builder()
        .with_reader(reader)
        .with_resource(resource)
        .build();
    global::set_meter_provider(provider.clone());
    provider
}
