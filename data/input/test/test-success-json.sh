#!/bin/bash

# Read the contents of example.json
json_data=$(cat example.json)

# Clean up the JSON data by removing any control characters
json_data=$(echo "$json_data" | tr -d '\000-\031')

# Escape any double quotes inside the JSON data to safely include it in the curl command
escaped_json_data=$(echo "$json_data" | sed 's/"/\\"/g')

# Sending the POST request with the json field as a string
response=$(curl -X POST http://localhost:8080/validate \
  -H "Content-Type: application/json" \
  -d "{
    \"protobuf\": \"MyMessage\",
    \"json\": \"$escaped_json_data\",
    \"field_check\": true,
    \"field_name\": \"key2\",
    \"field_value_check\": 42
  }")

# Print the response from the server
echo "Response: $response"

