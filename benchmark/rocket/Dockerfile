FROM rust:1.85-slim as builder
WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y pkg-config libssl-dev

COPY . .

# Build with maximum optimization
RUN cargo build --release --manifest-path benchmark/rocket/Cargo.toml

# Production image
FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update && apt-get install -y libssl3 ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/rocket-bench /usr/local/bin/rocket-bench

ENV ROCKET_PORT=8000
ENV ROCKET_ADDRESS=0.0.0.0
ENV ROCKET_LOG_LEVEL=off
EXPOSE 8000
CMD ["rocket-bench"]
