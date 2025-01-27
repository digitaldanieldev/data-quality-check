#!/bin/bash

# JSON payload (defined as a variable)
JSON_PAYLOAD='{
    "protobuf": "MyMessage",
    "json": {
        "key1": "example_value",
        "key2": 42,
        "key3": true
    },
    "json_escaped": false,
    "field_check": true,
    "field_name": "key2",
    "field_value_check": 42
}'

# URL to send the POST request
URL="http://localhost:8080/validate"

# Number of requests
NUM_REQUESTS=1000

# Initialize timing variables
WGET_TOTAL_TIME=0
CURL_TOTAL_TIME=0

echo "Running $NUM_REQUESTS requests for wget and curl..."

# Measure wget execution time over NUM_REQUESTS
for ((i=1; i<=NUM_REQUESTS; i++)); do
    WGET_START=$(date +%s.%N)
    wget --quiet \
        --method=POST \
        --header="Content-Type: application/json" \
        --body-data="$JSON_PAYLOAD" \
        -O - \
        $URL > /dev/null
    WGET_END=$(date +%s.%N)
    WGET_TOTAL_TIME=$(echo "$WGET_TOTAL_TIME + ($WGET_END - $WGET_START)" | bc)
done

# Measure curl execution time over NUM_REQUESTS
for ((i=1; i<=NUM_REQUESTS; i++)); do
    CURL_START=$(date +%s.%N)
    curl --silent \
        -X POST \
        -H "Content-Type: application/json" \
        -d "$JSON_PAYLOAD" \
        $URL > /dev/null
    CURL_END=$(date +%s.%N)
    CURL_TOTAL_TIME=$(echo "$CURL_TOTAL_TIME + ($CURL_END - $CURL_START)" | bc)
done

# Calculate averages
WGET_AVG_TIME=$(echo "$WGET_TOTAL_TIME / $NUM_REQUESTS" | bc -l)
CURL_AVG_TIME=$(echo "$CURL_TOTAL_TIME / $NUM_REQUESTS" | bc -l)

# Calculate requests per second
WGET_RPS=$(echo "$NUM_REQUESTS / $WGET_TOTAL_TIME" | bc -l)
CURL_RPS=$(echo "$NUM_REQUESTS / $CURL_TOTAL_TIME" | bc -l)

# Output results
echo
echo "Average Execution Time Over $NUM_REQUESTS Requests:"
echo "WGET: $WGET_AVG_TIME seconds (Requests Per Second: $WGET_RPS)"
echo "CURL: $CURL_AVG_TIME seconds (Requests Per Second: $CURL_RPS)"

