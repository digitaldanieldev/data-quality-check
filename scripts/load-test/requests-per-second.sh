#!/bin/bash

# Function to check if a value is a valid number
is_number() {
    [[ "$1" =~ ^[0-9]+(\.[0-9]+)?$ ]]
}

# Function to extract elapsed time and calculate statistics
extract_times() {
    logs_file=$1

    # Extract elapsed time values using grep
    time_values=($(grep -oP "Elapsed time: \K[0-9]+" "$logs_file"))

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
            echo "Warning: Invalid time value found: $time"
        fi
    done

    if ((count > 0)); then
        avg_time=$(echo "scale=2; $total_time / $count" | bc)
        echo "Number of requests: $count"
        echo "Total elapsed time: $total_time ms"
        echo "Average time per request: $avg_time ms"
        echo "Minimum time: $min_time ms"
        echo "Maximum time: $max_time ms"

        # Calculate requests per second
        rps=$(echo "scale=2; 1000 * $count / $total_time" | bc)
        echo "Requests per second: $rps"
    else
        echo "No valid time values found in the logs."
    fi
}

# Set the log file name
LOG_FILE="test_logs.txt"

# Check if the log file exists
if [[ -f "$LOG_FILE" ]]; then
    echo "Processing logs from $LOG_FILE..."
    extract_times "$LOG_FILE"
else
    echo "$LOG_FILE not found."
fi
