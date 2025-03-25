/* Licensed under the AGPL-3.0 License: https://www.gnu.org/licenses/agpl-3.0.html */

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use dynamic_message::{serialize_dynamic_message, populate_dynamic_message};
use prost_reflect::{DescriptorPool, DynamicMessage};
use prost_types::FileDescriptorSet;
use serde_json::json;
use std::fs::File;
use std::io::Read;

fn load_test_descriptor() -> FileDescriptorSet {
    let filename = "tests/example.pb";
    let mut file = File::open(filename).expect("Failed to open file");
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer).expect("Failed to read file content");
    prost::Message::decode(&*buffer).expect("Failed to decode .pb file")
}

fn benchmark_serialize_message(c: &mut Criterion) {
    let file_descriptor_set = load_test_descriptor();
    let mut descriptor_pool = DescriptorPool::new();
    descriptor_pool
        .add_file_descriptor_set(file_descriptor_set)
        .expect("Failed to add descriptor");

    let message_descriptor = descriptor_pool
        .get_message_by_name("MyMessage")
        .expect("Message not found");

    let mut dynamic_message = DynamicMessage::new(message_descriptor.clone());

    // Create a valid JSON object
    let json_value = json!({
        "key1": "test_value",
        "key2": 42,
        "key3": true
    });

    populate_dynamic_message(&mut dynamic_message, &message_descriptor, &json_value)
        .expect("Failed to populate dynamic message");

    c.bench_function("serialize_small_message", |b| {
        b.iter(|| serialize_dynamic_message(black_box(&mut dynamic_message)).expect("Serialization failed"));
    });
}


criterion_group!(
    benches,
    benchmark_serialize_message
);

criterion_main!(benches);
