#!/bin/bash

# Add debugging information at the start
#echo "Debug info:"
#echo "SERVER_IP: $SERVER_IP"
#echo "SERVER_PORT: $SERVER_PORT"
# env | grep SERVER

# Function to run the load test using curl
run_curl() {
    local run_amount=${RUN_AMOUNT}  # Get the environment variable
    for ((i=1; i<=run_amount; i++)); do
        echo "Run $i of $run_amount"
        start_time=$(date +%s%3N)

        # Load the static fields from message.json
        message_json=$(cat message.json)

        # Read the content of data.json
        data_json=$(cat data.json)

        # Inject the content of data.json into the "json" field of message.json
        final_json=$(echo "$message_json" | jq --argjson data "$data_json" '.json = $data')

        # Run the curl request with the final JSON payload
        curl -s -X POST -o /dev/null "http://$SERVER_IP:$SERVER_PORT/validate" \
            -H "Content-Type: application/json" \
            -d "$final_json"

        end_time=$(date +%s%3N)
        elapsed_time=$((end_time - start_time))
        echo "Curl test complete. Elapsed time: $elapsed_time ms."
    done
}

# Function to run the load test using wget
run_wget() {
    local run_amount=${RUN_AMOUNT}  # Get the environment variable
    for ((i=1; i<=run_amount; i++)); do
        echo "Run $i of $run_amount"
        start_time=$(date +%s%3N)

        # Load the static fields from message.json
        message_json=$(cat message.json)

        # Read the content of data.json
        data_json=$(cat data.json)

        # Inject the content of data.json into the "json" field of message.json
        final_json=$(echo "$message_json" | jq --argjson data "$data_json" '.json = $data')

        # Run the wget request with the final JSON payload
        wget --quiet --method=POST --header="Content-Type: application/json" \
            --body-data="$final_json" "http://$SERVER_IP:$SERVER_PORT/validate" -O /dev/null

        end_time=$(date +%s%3N)
        elapsed_time=$((end_time - start_time))
        echo "Wget test complete. Elapsed time: $elapsed_time ms."
    done
}

# Check if the correct flags are passed
if [[ "$1" == "--curl" ]]; then
    run_curl
elif [[ "$1" == "--wget" ]]; then
    run_wget
else
    echo "Usage: load-test.sh --curl or load-test.sh --wget"
    exit 1
fi

