#!/bin/bash

# Set the maximum number of open file descriptors (ulimit)
ulimit -n 20000

# Start the Docker Compose services in the background
docker compose -f docker-compose-load-test.yml up --build --scale load_test=3 -d

# Function to check if any container with the pattern "data-quality-check-load_test-" is running
check_containers_stopped() {
    # Get the list of running containers with the name pattern "data-quality-check-load_test"
    running_containers=$(docker ps --filter "name=data-quality-check-load_test" --format "{{.Names}}")
    
    # If there are any containers running, return true (1), else false (0)
    if [[ -z "$running_containers" ]]; then
        return 0  # No running containers, all containers are stopped
    else
        return 1  # There are still running containers
    fi
}

# Wait until all containers with the pattern "data-quality-check-load_test-" are stopped
echo "Waiting for all containers to stop..."
while ! check_containers_stopped; do
    # Sleep for a few seconds to avoid overwhelming the system
    sleep 5
done

echo "All containers are stopped. Proceeding with the next steps..."

# Initialize the total requests and total duration counters
total_requests=0
total_duration=0.0
valid_duration_containers=0

# Get the list of all containers (including stopped) that match the load_test name pattern
load_test_containers=$(docker ps -a --filter "name=data-quality-check-load_test" --format "{{.Names}}")

# Loop through each load_test container
for container in $load_test_containers; do
    echo "Processing container: $container"  # Output the container being processed

    # Fetch the logs for the container, suppressing all output (including errors)
    logs=$(docker logs "$container" 2>/dev/null)

    # Check if "Total requests" is in the logs
    if echo "$logs" | grep -q "Total requests"; then
        echo "'Total requests' found in container $container logs"  # Debug output
        
        # Extract all occurrences of "Total requests: <number>" from the logs
        matches_requests=$(echo "$logs" | grep -oP "Total requests: \K\d+")

        # Loop through each match found and add it to the total_requests
        for requests in $matches_requests; do
            # Clean up the requests variable to remove any unwanted characters (e.g., spaces or newlines)
            requests=$(echo "$requests" | tr -d '[:space:]')

            # Ensure requests is numeric and valid before adding to total_requests
            if [[ "$requests" =~ ^[0-9]+$ ]]; then
                # Convert requests to a number and add it to the total
                total_requests=$((total_requests + requests))
                echo "Extracted requests from container $container: $requests"  # Debug output
            else
                echo "Invalid 'Total requests' data found in container $container logs. Skipping."  # Debug output
            fi
        done
    else
        echo "No 'Total requests' found in container $container logs. Skipping."  # Debug output
    fi

    # Check if "Duration" is in the logs
    if echo "$logs" | grep -q "Duration"; then
        echo "'Duration' found in container $container logs"  # Debug output
        
        # Extract all occurrences of "Duration: <number>ms" or "Duration: <number>s" from the logs
        matches_duration=$(echo "$logs" | grep -oP "Duration: \K[0-9.]+(?=(ms|s))")

        # Loop through each match found and add it to the total_duration
        for duration in $matches_duration; do
            # Clean up the duration variable to remove any unwanted characters (e.g., spaces or newlines)
            duration=$(echo "$duration" | tr -d '[:space:]')

            # Check if the duration is in ms (milliseconds)
            if [[ "$logs" =~ Duration.*ms ]]; then
                # Convert ms to seconds (divide by 1000)
                duration=$(echo "$duration / 1000" | bc -l)
            fi

            # Ensure duration is numeric and valid before adding to total_duration
            if [[ "$duration" =~ ^[0-9.]+$ ]]; then
                # Convert duration to a number and add it to the total
                total_duration=$(echo "$total_duration + $duration" | bc)
                valid_duration_containers=$((valid_duration_containers + 1))  # Count valid containers with duration
                echo "Extracted duration from container $container: $duration"  # Debug output
            else
                echo "Invalid 'Duration' data found in container $container logs. Skipping."  # Debug output
            fi
        done
    else
        echo "No 'Duration' found in container $container logs. Skipping."  # Debug output
    fi
done

# Calculate requests per second (RPS) based on the total duration divided by valid containers
if (( valid_duration_containers > 0 )); then
    # Calculate the average duration per container with a valid duration
    average_duration=$(echo "$total_duration / $valid_duration_containers" | bc -l)
    # Calculate RPS: total_requests divided by the average duration (in seconds)
    rps=$(echo "$total_requests / $average_duration" | bc -l)
else
    rps=0
fi

# Only print the aggregated result if there are valid total requests and total duration
echo "Aggregated Total Requests: $total_requests"
echo "Aggregated Total Duration (in seconds): $total_duration"
echo "Requests per second (RPS): $rps"

# Shut down the Docker Compose services after the script finishes
docker compose -f docker-compose-load-test.yml down

