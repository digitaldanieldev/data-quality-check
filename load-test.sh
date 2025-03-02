#!/bin/bash

SCRIPT_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )"

# Change to target/release directory
cd "$SCRIPT_DIR/target/release" || { echo "Error: Could not change to release directory"; exit 1; }

# Start all services with nohup
nohup ./data-quality-server > data-quality-server.log 2>&1 &
sleep 1
nohup ./config-producer > config-producer.log 2>&1 &
sleep 1
nohup ./load-test > load-test.log 2>&1 &

echo "All services started in background mode."
echo "Logs are available in:"
echo "- data-quality-server.log"
echo "- config-producer.log"
echo "- load-test.log"

# Display combined logs
echo "\nStarting log viewer..."
echo "Press Ctrl+C to exit log view mode"
tail -f *.log
