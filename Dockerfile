# Builder Stage
FROM rust:slim-buster AS builder
ENV USER=root
ENV CARGO_HOME=/usr/local/cargo
ENV RUSTUP_HOME=/usr/local/rustup

RUN apt-get update && apt-get upgrade -y && \
    apt-get install -y \
    build-essential cmake curl exuberant-ctags fzf g++ gettext git gnupg htop \
    libc-dev libcurl4-openssl-dev libegl1-mesa libfreetype6-dev libgl1-mesa-glx libssl-dev libtool-bin libx11-xcb1 libxft-dev \
    lua5.1 luajit luarocks make netcat ninja-build openssh-server pkg-config ripgrep \
    software-properties-common sudo unzip x11-utils xclip xdg-desktop-portal xdg-utils zlib1g-dev \
    && rm -rf /var/lib/apt/lists/*


# Set up build directory and copy all files
WORKDIR /app
COPY . .

# Build both binaries
RUN cargo build --release

# Runtime Stage for data_quality_server
FROM debian:buster-slim AS data_quality
COPY --from=builder /app/target/release/data-quality-server /data-quality-server
ENTRYPOINT ["/data-quality-server"]

# Runtime Stage for config_producer
FROM debian:buster-slim AS config_producer

RUN apt-get update && apt-get upgrade -y && \
    apt-get install -y \
    autoconf libtool m4 protobuf-compiler unzip 

COPY --from=builder /app/target/release/config-producer-proto /config-producer-proto
ENTRYPOINT ["/config-producer-proto"]
