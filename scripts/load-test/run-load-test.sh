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

# Log directory where container logs are stored
LOGS_DIR="container_logs"

# Project prefix
PROJECT_NAME="load-test-"

# Check if valid service name is provided
echo "Checking service name: $SERVICE_NAME"
if [[ "$SERVICE_NAME" != "load-test-wget" && "$SERVICE_NAME" != "load-test-curl" ]]; then
    echo "Invalid service name. Please use 'load-test-wget' or 'load-test-curl'."
    exit 1
fi

# Stop any running containers before starting new ones
echo "Stopping any running containers..."
docker-compose down

# Run the load test with the specified service and scale size
echo "Running docker-compose with $SERVICE_NAME scaled to $SCALE_SIZE replicas..."
docker-compose up $SERVICE_NAME --scale $SERVICE_NAME=$SCALE_SIZE -d

echo "Waiting for the load test to complete..."
# Wait until all containers are stopped
while true; do
    # Check the status of the containers for the service
    container_status=$(docker-compose ps -q $SERVICE_NAME | xargs docker inspect --format '{{.State.Status}}' | grep -c -e "exited" -e "dead")
    
    # If the number of exited or dead containers matches the scale size, break the loop
    if [ "$container_status" -eq "$SCALE_SIZE" ]; then
        echo "All containers have stopped."
        break
    else
        echo "Waiting for the load test to complete..."
        sleep 2
    fi
done

# Create the container_logs directory if it doesn't exist
echo "Creating log directory: $LOGS_DIR"
mkdir -p "$LOGS_DIR"

# Delete all existing log files in the container_logs directory before fetching new ones
echo "Deleting old log files in $LOGS_DIR..."
rm -f "$LOGS_DIR"/*.log

# Fetch the logs for each container and save them in the logs directory
echo "Fetching logs for each container in $SERVICE_NAME..."
for i in $(seq 1 $SCALE_SIZE); do
    CONTAINER_NAME="${PROJECT_NAME}${SERVICE_NAME}-${i}"
    LOG_FILE="$LOGS_DIR/$SERVICE_NAME-$i.log"
    
    echo "Fetching log for $CONTAINER_NAME..."
    docker logs "$CONTAINER_NAME" > "$LOG_FILE" 2>/dev/null
    echo "Log for $CONTAINER_NAME saved to $LOG_FILE"
done

# Run the requests-per-second script to process the logs
echo "Running requests-per-second script on logs in $LOGS_DIR..."
./requests-per-second.sh "$LOGS_DIR"

# Optionally, stop the containers after the test
echo "Stopping containers after the load test..."
docker-compose down

echo "Removing log files..."
rm -f "$LOGS_DIR"/*.log

echo "Load test completed and results processed."
