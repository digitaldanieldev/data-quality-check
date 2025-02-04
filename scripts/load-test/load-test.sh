#!/bin/bash

# File to hold the JSON data
DATA_FILE="data.json"

# Number of requests
NUM_REQUESTS=10000
URL="http://localhost:8080/validate"

# Function to run wget requests
function run_wget {
    echo "Running wget..."
    start_time=$(date +%s%3N) 

    for ((i=1; i<=NUM_REQUESTS; i++)); do
        wget --quiet --method=POST --header="Content-Type: application/json" --body-file=$DATA_FILE -O - $URL > /dev/null
    done

    end_time=$(date +%s%3N) 
    elapsed_time=$((end_time - start_time)) 
    rps=$(echo "scale=2; $NUM_REQUESTS / ($elapsed_time / 1000)" | bc) 
    echo "Wget test complete. Elapsed time: $elapsed_time ms. Requests per second: $rps."
}

# Function to run curl requests
function run_curl {
    echo "Running curl..."
    start_time=$(date +%s%3N) 

    for ((i=1; i<=NUM_REQUESTS; i++)); do
        curl --silent -X POST -H "Content-Type: application/json" -d "@$DATA_FILE" $URL > /dev/null
    done

    end_time=$(date +%s%3N) 
    elapsed_time=$((end_time - start_time)) 
    rps=$(echo "scale=2; $NUM_REQUESTS / ($elapsed_time / 1000)" | bc) 
    echo "Curl test complete. Elapsed time: $elapsed_time ms. Requests per second: $rps."
}

# Parse flags
if [[ "$1" == "--wget" ]]; then
    echo "Starting speed test with wget..."
    run_wget
elif [[ "$1" == "--curl" ]]; then
    echo "Starting speed test with curl..."
    run_curl
else
    echo "Please provide either --wget or --curl to choose the tool to run."
    exit 1
fi

