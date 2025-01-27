#[cfg(test)]
mod tests {
    use super::*;
    use dynamic_message::{populate_dynamic_message, serialize_dynamic_message};
    use prost_reflect::{DescriptorPool, DynamicMessage};
    use prost_types::FileDescriptorSet;
    use serde_json::json;
    use std::fs::File;
    use std::io::Read;
    use std::path::Path;

    // Helper function to load a test .pb file
    fn load_test_descriptor() -> Result<FileDescriptorSet, String> {
        let filename = "tests/example.pb";
        let mut file = File::open(filename).map_err(|e| format!("Failed to open file: {:?}", e))?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)
            .map_err(|e| format!("Failed to read file content: {:?}", e))?;
        prost::Message::decode(&*buffer).map_err(|e| format!("Failed to decode .pb file: {:?}", e))
    }

    // Test for populating a dynamic message with a valid JSON object
    #[test]
    fn test_populate_dynamic_message_valid() {
        let file_descriptor_set = load_test_descriptor().expect("Failed to load test descriptor");
        let mut descriptor_pool = DescriptorPool::new();
        descriptor_pool
            .add_file_descriptor_set(file_descriptor_set)
            .expect("Failed to add descriptor");

        let message_descriptor = descriptor_pool
            .get_message_by_name("MyMessage")
            .expect("Message not found");

        let mut dynamic_message = DynamicMessage::new(message_descriptor.clone());

        // Create a valid JSON object matching the expected fields
        let json_value = json!({
            "key1": "test_value",
            "key2": 42,
            "key3": true
        });

        let result =
            populate_dynamic_message(&mut dynamic_message, &message_descriptor, &json_value);

        assert!(
            result.is_ok(),
            "Expected successful population of dynamic message"
        );
    }

    // Test for populating a dynamic message with invalid JSON (type mismatch)
    #[test]
    fn test_populate_dynamic_message_invalid_type() {
        let file_descriptor_set = load_test_descriptor().expect("Failed to load test descriptor");
        let mut descriptor_pool = DescriptorPool::new();
        descriptor_pool
            .add_file_descriptor_set(file_descriptor_set)
            .expect("Failed to add descriptor");

        let message_descriptor = descriptor_pool
            .get_message_by_name("MyMessage") // Replace with the actual message name in the .pb file
            .expect("Message not found");

        let mut dynamic_message = DynamicMessage::new(message_descriptor.clone());

        // Create a JSON object with a type mismatch (e.g., string instead of integer)
        let json_value = json!({
            "field1": "test_value",
            "field2": "invalid_type_instead_of_integer" // This will cause an error
        });

        let result =
            populate_dynamic_message(&mut dynamic_message, &message_descriptor, &json_value);

        assert!(result.is_err(), "Expected error due to type mismatch");
    }

    // Test for serializing a populated dynamic message
    #[test]
    fn test_serialize_dynamic_message() {
        let file_descriptor_set = load_test_descriptor().expect("Failed to load test descriptor");
        let mut descriptor_pool = DescriptorPool::new();
        descriptor_pool
            .add_file_descriptor_set(file_descriptor_set)
            .expect("Failed to add descriptor");

        let message_descriptor = descriptor_pool
            .get_message_by_name("MyMessage")
            .expect("Message not found");

        let mut dynamic_message = DynamicMessage::new(message_descriptor.clone());

        // Create a valid JSON object for population
        let json_value = json!({
            "key1": "test_value",
            "key2": 42,
            "key3": true
        });

        populate_dynamic_message(&mut dynamic_message, &message_descriptor, &json_value)
            .expect("Failed to populate dynamic message");

        // Serialize the dynamic message
        let serialized = serialize_dynamic_message(&mut dynamic_message)
            .expect("Failed to serialize dynamic message");

        // Validate the serialization result
        assert!(
            !serialized.is_empty(),
            "Expected non-empty serialized message"
        );
    }

    // Test for serializing an empty dynamic message
    #[test]
    fn test_serialize_empty_dynamic_message() {
        let file_descriptor_set = load_test_descriptor().expect("Failed to load test descriptor");
        let mut descriptor_pool = DescriptorPool::new();
        descriptor_pool
            .add_file_descriptor_set(file_descriptor_set)
            .expect("Failed to add descriptor");

        let message_descriptor = descriptor_pool
            .get_message_by_name("MyMessage")
            .expect("Message not found");

        let mut dynamic_message = DynamicMessage::new(message_descriptor.clone());

        // Serialize the empty dynamic message
        let serialized = serialize_dynamic_message(&mut dynamic_message)
            .expect("Failed to serialize empty dynamic message");

        // Ensure that serialization produces a result
        assert!(
            !serialized.is_empty(),
            "Expected non-empty serialized message for empty dynamic message"
        );
    }

    // Test for invalid field (field does not exist in the descriptor)
    #[test]
    fn test_populate_dynamic_message_invalid_field() {
        let file_descriptor_set = load_test_descriptor().expect("Failed to load test descriptor");
        let mut descriptor_pool = DescriptorPool::new();
        descriptor_pool
            .add_file_descriptor_set(file_descriptor_set)
            .expect("Failed to add descriptor");

        let message_descriptor = descriptor_pool
            .get_message_by_name("MyMessage")
            .expect("Message not found");

        let mut dynamic_message = DynamicMessage::new(message_descriptor.clone());

        // Create a JSON object with an invalid field name
        let json_value = json!({
            "invalid_field": "test_value"
        });

        let result =
            populate_dynamic_message(&mut dynamic_message, &message_descriptor, &json_value);

        assert!(result.is_err(), "Expected error due to invalid field name");
    }
}
