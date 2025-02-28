# Builder Stage
FROM rust:slim-buster AS builder
#ENV USER=root
ENV RUSTFLAGS="-C target-feature=+crt-static -C link-arg=-static"
ENV CARGO_HOME=/usr/local/cargo
ENV RUSTUP_HOME=/usr/local/rustup

# Install musl libc for static linking
RUN apk add --no-cache musl-dev gcc musl-crt

# Set up build directory and copy all files
WORKDIR /app
COPY . .

# Build both binaries
RUN cargo build --release

# Runtime Stage for data_quality_server
FROM debian:buster-slim AS data_quality
COPY --from=builder /app/target/release/data_quality_server /data_quality_server
COPY .env .env
ENTRYPOINT ["/data_quality_server"]

# Runtime Stage for config_producer
FROM debian:buster-slim AS config_producer
COPY --from=builder /app/target/release/config_producer_proto /config_producer_proto
COPY .env .env
ENTRYPOINT ["/config_producer_proto"]
