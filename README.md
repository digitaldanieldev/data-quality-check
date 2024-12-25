# Data-quality-server

## Enable metrics
./data-quality-server --enable-metrics
./data-quality-server --worker-threads 4 --enable-metrics


# Config-producer-proto

## Run Once (One-time processing)

This will process the .proto files and send them to the server once.

./config-producer-proto

## Run the program in a loop.

Check for .proto file updates every 30 seconds.
./config-producer-proto --loop --interval 30

## Check is a message is valid JSON

## Check if JSON can be serialized using MyMessage protobuf definition
curl -X POST http://localhost:8080/validate \
  -H "Content-Type: application/json" \
  -d '{
    "n": "MyMessage", 
    "json": "{\"key1\": \"example_value\", \"key2\": 42, \"key3\": true}"}'

## Check if JSON can be serialized using MyMessage protobuf definition and validate that field key2 contains the number 42
curl -X POST http://localhost:8080/validate \
  -H "Content-Type: application/json" \
  -d '{
    "n": "MyMessage", 
    "json": "{\"key1\": \"example_value\", \"key2\": 42, \"key3\": true}", 
    "validate_field": true,
    "field_name": "key2",
    "field_value_check": 42  }'

curl -X POST http://localhost:8080/validate -d '{"key1": "example_value", "key2": 42, "key3": true}'

curl -X POST http://localhost:8080/validate -d '{
    "json": {
        "key1": "example_value",
        "key2": 42,
        "key3": true
    },
    "json_check": true
}' -H "Content-Type: application/json"

./data-quality-server --json '{"key1": "value1", "key2": 42}'