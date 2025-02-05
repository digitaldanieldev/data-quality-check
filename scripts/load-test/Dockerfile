# Dockerfile
FROM ubuntu:latest

ARG SERVER_IP
ARG SERVER_PORT

ENV SERVER_IP=${SERVER_IP}
ENV SERVER_PORT=${SERVER_PORT}

# Install wget, curl, bc (for math operations), jq
RUN apt-get update && apt-get install -y \
    wget \
    curl \
    bc \
    jq \
    && rm -rf /var/lib/apt/lists/*

# Add the load-test script into the container
COPY load-test.sh /load-test.sh
COPY data.json /data.json
COPY message.json /message.json

# Make the script executable
RUN chmod +x /load-test.sh

# Set the default command to run the load test script
CMD ["/load-test.sh", "--curl"]

