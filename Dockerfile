# Use the official Rust image as a build environment
FROM rust:1.85-slim-bookworm as builder

# Set the working directory
WORKDIR /app

# Copy the source code into the container
COPY . .

# Build the Rust application in release mode
RUN cargo build --release

# Use a minimal base image for the final container
FROM debian:bookworm-slim

# Install necessary dependencies
RUN apt-get update && apt-get install -y \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

# Set the working directory
WORKDIR /app

# Copy the built binary from the builder stage
COPY --from=builder /app/target/release/spaceapipoller /app/spaceapipoller

# Copy any additional files or configurations needed for your application
COPY config.toml /app/config.toml

# Set the entry point to your Rust application
ENTRYPOINT ["./spaceapipoller"]