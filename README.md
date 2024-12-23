# Data-quality-server

## Enable metrics
./data-quality-server --enable-metrics

# Config-producer-proto

## Run Once (One-time processing)

This will process the .proto files and send them to the server once.

./config-producer-proto

## Run the program in a loop.

Check for .proto file updates every 30 seconds.
./config-producer-proto --loop --interval 30


