#!/bin/bash

# Function to check if a value is a valid number
is_number() {
    [[ "$1" =~ ^[0-9]+(\.[0-9]+)?$ ]]
}

# Function to extract elapsed time and calculate statistics per container
extract_times_per_container() {
    logs_file=$1

    # Extract container names (first column)
    container_names=($(awk '{print $1}' "$logs_file" | sort | uniq))

    total_rps_all_containers=0
    total_requests_all_containers=0
    total_time_all_containers=0

    # Loop through each unique container name
    for container in "${container_names[@]}"; do
        echo "Processing logs for container: $container"

        # Extract elapsed time values for this container
        time_values=($(grep "$container" "$logs_file" | grep -oP "Elapsed time: \K[0-9]+"))

        total_time=0
        count=0
        min_time=999999
        max_time=0

        for time in "${time_values[@]}"; do
            time=$(echo "$time" | xargs)

            if is_number "$time"; then
                # Update total
                total_time=$(echo "$total_time + $time" | bc)
                ((count++))

                # Update min
                if ((time < min_time)); then
                    min_time=$time
                fi

                # Update max
                if ((time > max_time)); then
                    max_time=$time
                fi
            else
                echo "Warning: Invalid time value found for container $container: $time"
            fi
        done

        if ((count > 0)); then
            avg_time=$(echo "scale=2; $total_time / $count" | bc)
            echo "Container $container - Number of requests: $count"
            echo "Container $container - Total elapsed time: $total_time ms"
            echo "Container $container - Average time per request: $avg_time ms"
            echo "Container $container - Minimum time: $min_time ms"
            echo "Container $container - Maximum time: $max_time ms"

            # Calculate requests per second for this container
            rps=$(echo "scale=2; 1000 * $count / $total_time" | bc)
            echo "Container $container - Requests per second: $rps"

            # Update total for all containers
            total_rps_all_containers=$(echo "$total_rps_all_containers + $rps" | bc)
            total_requests_all_containers=$((total_requests_all_containers + count))
            total_time_all_containers=$(echo "$total_time_all_containers + $total_time" | bc)
        else
            echo "No valid time values found in logs for container $container."
        fi
        echo "-----------------------------------"
    done

    # Now calculate the total requests per second across all containers by summing individual RPS
    echo "Total Requests per second across all containers: $total_rps_all_containers"
}

# Set the log file name
LOG_FILE="test_logs.txt"

# Check if the log file exists
if [[ -f "$LOG_FILE" ]]; then
    echo "Processing logs from $LOG_FILE..."
    extract_times_per_container "$LOG_FILE"
else
    echo "$LOG_FILE not found."
fi

