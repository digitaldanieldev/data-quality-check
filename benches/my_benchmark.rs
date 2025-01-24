use criterion::{black_box, Criterion};
use prost_reflect::{
    DynamicMessage, MessageDescriptor, SerializeOptions,
};
use serde_json::Value as JsonValue;
use tracing::{debug, info};

#[tracing::instrument]
pub fn criterion_benchmark(c: &mut Criterion) {
    // Mock data setup for the test
    let message_descriptor = MessageDescriptor::default(); // You'd usually load the actual descriptor here
    let json_value: JsonValue = serde_json::json!({
        "key1": "example_value",
        "key2": 42,
        "key3": true,
    });

    // Create a DynamicMessage object
    let mut dynamic_message = DynamicMessage::new(message_descriptor);

    // Populate the dynamic message with the sample JSON data
    populate_dynamic_message(&mut dynamic_message, &message_descriptor, &json_value).unwrap();

    // Benchmark the serialization of the dynamic message
    c.bench_function("serialize_dynamic_message", |b| {
        b.iter(|| serialize_dynamic_message(black_box(&mut dynamic_message.clone())))
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);