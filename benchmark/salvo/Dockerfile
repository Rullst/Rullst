FROM rust:1.85-slim as builder
WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y pkg-config libssl-dev

COPY . .

# Build with maximum optimization
RUN cargo build --release --manifest-path benchmark/salvo/Cargo.toml

# Production image
FROM debian:bookworm-slim
WORKDIR /app
RUN apt-get update && apt-get install -y libssl3 ca-certificates && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/salvo-bench /usr/local/bin/salvo-bench

EXPOSE 8000
CMD ["salvo-bench"]
