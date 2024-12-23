#!/bin/bash

# Read the contents of example.json
json_data=$(cat example.json)

# Sending a POST request with the contents of example.json included in the payload
response=$(curl -X POST http://localhost:8080/validate \
  -H "Content-Type: application/json" \
  -d '{
    "n": "MyMessage",
    "json": '"$json_data"',
    "validate_field": true,
    "field_name": "key2",
    "field_value_check": 42
  }')

# Print the response from the server
echo "Response: $response"
