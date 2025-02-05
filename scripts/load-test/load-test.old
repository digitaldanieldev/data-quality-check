#!/bin/bash

# Add debugging information at the start
#echo "Debug info:"
#echo "SERVER_IP: $SERVER_IP"
#echo "SERVER_PORT: $SERVER_PORT"
# env | grep SERVER

# Function to run the load test using curl
run_curl() {
    #echo "Running curl load test..."
    start_time=$(date +%s%3N)

    # Load the static fields from message.json
    message_json=$(cat message.json)
    #echo "message.json content: $message_json"  # Debug line

    # Read the content of data.json
    data_json=$(cat data.json)
    #echo "data.json content: $data_json"  # Debug line

    # Inject the content of data.json into the "json" field of message.json
    final_json=$(echo "$message_json" | jq --argjson data "$data_json" '.json = $data')
    #echo "Final JSON: $final_json"  # Debug line

    # Echo the curl command before running it
    #echo "Executing curl command to: http://$SERVER_IP:$SERVER_PORT/validate"

    # Run the curl request with the final JSON payload
    # -s -> silent but print response
    curl -s -X POST -o /dev/null "http://$SERVER_IP:$SERVER_PORT/validate" \
        -H "Content-Type: application/json" \
        -d "$final_json"

    end_time=$(date +%s%3N)
    elapsed_time=$((end_time - start_time))
    echo "Curl test complete. Elapsed time: $elapsed_time ms."
}
# Function to run the load test using wget
run_wget() {
    #echo "Running wget load test..."
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

