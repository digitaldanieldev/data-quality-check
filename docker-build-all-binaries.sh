#!/bin/bash

OUTPUT_DIR="$(pwd)/docker-builds"

mkdir -p "$OUTPUT_DIR"
echo "Output directory created at $OUTPUT_DIR."

echo "Building and starting services using Docker Compose..."
docker compose up --build -d

declare -A CONTAINERS_BINS=(
    ["config_producer_proto"]="config-producer-proto"
    ["data_quality_server"]="data-quality-server"
    ["load_test_service"]="load-test"
)

echo "Waiting for containers to be ready..."
sleep 5

copy_binary() {
    local container_name="$1"
    local binary_name="$2"
    echo "Attempting to copy $binary_name from $container_name..."

    if docker cp "$container_name:/$binary_name" "$OUTPUT_DIR/$binary_name"; then
        echo "Successfully copied $binary_name!"
        return 0
    else
        echo "Failed to copy $binary_name, retrying..."
        return 1
    fi
}

MAX_ATTEMPTS=20
ATTEMPT=1

echo "Copying binaries from containers to the output directory..."
while [ $ATTEMPT -le $MAX_ATTEMPTS ]; do
    echo -e "\nAttempt $ATTEMPT/$MAX_ATTEMPTS"
    success=true
    
    for container in "${!CONTAINERS_BINS[@]}"; do
        binary="${CONTAINERS_BINS[$container]}"
        
        if ! copy_binary "$container" "$binary"; then
            success=false
        fi
    done
    
    if $success; then
        break
    fi

    ATTEMPT=$((ATTEMPT + 1))
    sleep 2 
done

echo -e "\nFinal Status:"
for container in "${!CONTAINERS_BINS[@]}"; do
    binary="${CONTAINERS_BINS[$container]}"
    if [ -f "$OUTPUT_DIR/$binary" ]; then
        echo "- Successfully copied: $binary"
    else
        echo "- Failed to copy: $binary"
    fi
done

echo "Cleaning up the containers..."
docker compose down

# Done
echo "Done! The binaries have been copied to $OUTPUT_DIR."
