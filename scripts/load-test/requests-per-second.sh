#!/bin/bash

# Function to check if a value is a valid number
is_number() {
   
    [[ "$1" =~ ^[0-9]+(\.[0-9]+)?$ ]]
}

# Extract RPS from the logs and calculate the average
extract_rps() {
    logs_file=$1
   
    rps_values=($(grep -oP "Requests per second: \K[0-9.]+" "$logs_file"))

    total_rps=0
    count=0

    for rps in "${rps_values[@]}"; do
       
        rps=$(echo "$rps" | xargs)

       
        rps=$(echo "$rps" | sed 's/\.$//')

       
        if is_number "$rps"; then
            total_rps=$(echo "$total_rps + $rps" | bc)
            ((count++))
        else
            echo "Warning: Invalid RPS value found: $rps"
        fi
    done

    if ((count > 0)); then
        avg_rps=$(echo "scale=2; $total_rps / $count" | bc)
        echo "Total Requests per second: $total_rps"
        echo "Average Requests per second: $avg_rps"
    else
        echo "No valid RPS values found in the logs."
    fi
}

# Set the log file names as parameters here
WGET_LOG_FILE="load-test-curl_logs.txt" 
CURL_LOG_FILE="load-test-wget_logs.txt" 

# Check if the wget log file exists
if [[ -f "$WGET_LOG_FILE" ]]; then
    echo "Processing wget logs from $WGET_LOG_FILE..."
    extract_rps "$WGET_LOG_FILE"
else
    echo "$WGET_LOG_FILE not found."
fi

# Check if the curl log file exists
if [[ -f "$CURL_LOG_FILE" ]]; then
    echo "Processing curl logs from $CURL_LOG_FILE..."
    extract_rps "$CURL_LOG_FILE"
else
    echo "$CURL_LOG_FILE not found."
fi

