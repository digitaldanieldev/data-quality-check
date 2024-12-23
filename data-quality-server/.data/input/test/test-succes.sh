#!/bin/bash

# Sending a POST request to validate the JSON
response=$(curl -X POST http://localhost:8080/validate \
  -H "Content-Type: application/json" \
  -d '{
    "n": "MyMessage", 
    "json": "{\"key1\": \"example_value\", \"key2\": 42, \"key3\": true}", 
    "validate_field": true,
    "field_name": "key2",
    "field_value_check": 42  }')

# Print the response from the server
echo "Response: $response"
