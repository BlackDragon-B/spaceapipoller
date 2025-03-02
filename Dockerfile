FROM rust:1.85-slim-bookworm as builder

WORKDIR /app

COPY . .

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

COPY --from=builder /app/target/release/spaceapipoller /app/spaceapipoller

ENTRYPOINT ["./spaceapipoller"]