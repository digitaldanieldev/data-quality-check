# Use a lightweight image that has curl and wget installed
FROM ubuntu:latest

# Install wget, curl, and bc (for math operations)
RUN apt-get update && apt-get install -y \
    wget \
    curl \
    bc \
    && rm -rf /var/lib/apt/lists/*

# Add the speed test script into the container
ADD load-test.sh /usr/local/bin/load-test.sh

# Make the script executable
RUN chmod +x /usr/local/bin/load-test.sh

# Set the default command to run the speed test script
# Default to wget
CMD ["/usr/local/bin/load-test.sh", "--wget"]

