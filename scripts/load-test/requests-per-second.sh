#!/bin/bash

# Function to check if a value is a valid number
is_number() {
    [[ "$1" =~ ^[0-9]+(\.[0-9]+)?$ ]]
}

# Function to extract elapsed time and calculate statistics per container
extract_times_per_container() {
    logs_dir=$1

    # Get a list of all the log files in the logs directory
    container_files=($(ls "$logs_dir"/*.log))

    total_rps_all_containers=0
    total_requests_all_containers=0
    total_time_all_containers=0

    # Loop through each container log file
    for log_file in "${container_files[@]}"; do
        container_name=$(basename "$log_file" .log)
        echo "Processing logs for container: $container_name"

        # Extract total test run time and total number of requests for this container
        total_run_time=$(grep -oP "Total test run time: \K[0-9]+" "$log_file")
        total_requests=$(grep -oP "Run [0-9]+ of [0-9]+" "$log_file" | wc -l)

        if is_number "$total_run_time" && is_number "$total_requests"; then
            # Calculate requests per second (RPS) for this container
            rps=$(echo "scale=2; 1000 * $total_requests / $total_run_time" | bc)
            echo "Container $container_name - Total test run time: $total_run_time ms"
            echo "Container $container_name - Number of requests: $total_requests"
            echo "Container $container_name - Requests per second: $rps"

            # Update total for all containers
            total_rps_all_containers=$(echo "$total_rps_all_containers + $rps" | bc)
            total_requests_all_containers=$((total_requests_all_containers + total_requests))
            total_time_all_containers=$(echo "$total_time_all_containers + $total_run_time" | bc)
        else
            echo "Warning: Invalid total run time or total requests found in container $container_name logs."
        fi
        echo "-----------------------------------"
    done

    # Now calculate the total requests per second across all containers by summing individual RPS
    echo "Total Requests per second across all containers: $total_rps_all_containers"
    echo "Total Requests made across all containers: $total_requests_all_containers"
    echo "Total elapsed time across all containers: $total_time_all_containers ms"
}

# Check if the logs directory exists
LOGS_DIR=$1
if [[ -d "$LOGS_DIR" ]]; then
    echo "Processing logs from all files in $LOGS_DIR..."
    extract_times_per_container "$LOGS_DIR"
else
    echo "Directory $LOGS_DIR not found."
    exit 1
fi
