FROM ubuntu:latest

# Define the build-time variables (ARG)
ARG RUN_AMOUNT=1000
ARG SERVER_IP
ARG SERVER_PORT

# Set environment variables for runtime
ENV RUN_AMOUNT=${RUN_AMOUNT}
ENV SERVER_IP=${SERVER_IP}
ENV SERVER_PORT=${SERVER_PORT}

# Install necessary dependencies
RUN apt-get update && apt-get install -y \
    wget \
    curl \
    bc \
    jq \
    && rm -rf /var/lib/apt/lists/*

# Copy the load-test script and necessary files into the container
COPY load-test.sh /load-test.sh
COPY data.json /data.json
COPY message.json /message.json

# Make the script executable
RUN chmod +x /load-test.sh

# Set the default command to run the load test script with the passed arguments
CMD ["/load-test.sh", "--curl", "$RUN_AMOUNT"]

