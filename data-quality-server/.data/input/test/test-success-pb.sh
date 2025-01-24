#!/bin/bash

# Sending a POST request to validate the JSON
response=$(curl -X POST http://localhost:8080/validate \
  -H "Content-Type: application/json" \
  -d '{
    "protobuf": "MyMessage", 
    "json": "{\"key1\": \"example_value\", \"key2\": 42, \"key3\": true}" }')

# Print the response from the server
echo "Response: $response"