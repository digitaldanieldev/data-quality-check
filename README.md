
./config-producer-proto --loop --interval 2

curl -X POST http://localhost:8080/validate \
     -H "Content-Type: application/json" \
     -d '{"n": "MyMessage", "json": "{\"key1\": \"example_value\", \"key2\": 42, \"key3\": true}"}'

curl -X POST http://localhost:8080/validate \
    -H "Content-Type: application/json" \
    -d '{
        "n": "MyMessage",
        "json": "{\"key1\": \"example_value\", \"key2\": 42, \"key3\": true}",
        "validate_field": true,
        "field_name": "key1",
        "forbidden_word": "example"
    }'

curl -X POST http://localhost:8080/validate \
    -H "Content-Type: application/json" \
    -d '{
        "n": "MyMessage",
        "json": "{\"key1\": \"example_value\", \"key2\": 42, \"key3\": true}",
        "validate_field": false
    }'

curl -X POST http://localhost:8080/validate      -H "Content-Type: application/json"      -d '{"n": "MyMessage", "json": "{\"key1\": \"example_value\", \"key2\": 42, \"key3\": true}", "validate_field": true, "field_name": "key1", "forbidden_word": "example_value" }'

curl -X POST http://localhost:8080/validate \
     -H "Content-Type: application/json" \
     -d '{"n": "MyMessage", "json": "{\"key1\": \"example_value\", \"key2\": 42, \"key3\": true}", "validate_field": true, "field_name": "key1", "forbidden_word": "example_value" }'

curl -X POST http://localhost:8080/validate \
  -H "Content-Type: application/json" \
  -d '{
    "n": "MyMessage", 
    "json": {"key1": "example_value", "key2": 42, "key3": true}, 
    "validate_field": true
  }'