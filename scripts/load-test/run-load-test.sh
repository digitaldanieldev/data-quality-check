#!/bin/bash

# Check if correct number of arguments are provided
if [ "$#" -ne 2 ]; then
    echo "Usage: $0 <service_name> <scale_size>"
    echo "Example: $0 load-test-curl 5"
    exit 1
fi

# Parameters from CLI arguments
SERVICE_NAME=$1      # Service name (e.g., load-test-wget or load-test-curl)
SCALE_SIZE=$2        # Scale size (e.g., 5)

# Log file names
LOG_FILE="test_logs.txt"

# Check if valid service name is provided
if [[ "$SERVICE_NAME" != "load-test-wget" && "$SERVICE_NAME" != "load-test-curl" ]]; then
    echo "Invalid service name. Please use 'load-test-wget' or 'load-test-curl'."
    exit 1
fi

# Stop any running containers before starting new ones
echo "Stopping containers..."
docker-compose down

# Run the load test with the specified service and scale size
echo "Running docker-compose with $SERVICE_NAME scaled to $SCALE_SIZE replicas..."
docker-compose up $SERVICE_NAME --scale $SERVICE_NAME=$SCALE_SIZE -d

# Give some time for the containers to start and perform the test
echo "Waiting for the containers to start and complete the load test..."
sleep 10

# Remove any existing *_logs.txt files
echo "Removing old log files..."
rm ./test_logs.txt

# Fetch the logs for the specified service and write them to the log file
echo "Fetching logs for $SERVICE_NAME..."
docker-compose logs $SERVICE_NAME > "$LOG_FILE"

# Run the requests-per-second script to process the logs
#echo "Running requests-per-second script..."
./requests-per-second.sh "$LOG_FILE"

# Optionally, stop the containers after the test
echo "Stopping containers..."
docker-compose down

echo "Removing log files..."
rm ./test_logs.txt

echo "Load test completed and results processed."
