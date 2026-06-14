# Build stage
FROM rust:1.96-slim-bookworm AS builder
WORKDIR /usr/src/rullst

# Copy workspace files required for build
COPY Cargo.toml Cargo.lock ./
COPY rullst ./rullst
COPY rullst-macros ./rullst-macros
COPY cargo-rullst ./cargo-rullst
COPY examples ./examples
COPY benchmark/rullst ./benchmark/rullst

# Build rullst benchmark in release mode
WORKDIR /usr/src/rullst/benchmark/rullst
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim
WORKDIR /app
ENV APP_ENV=production
COPY --from=builder /usr/src/rullst/benchmark/rullst/target/release/bench-rullst /app/bench-rullst
EXPOSE 3000
CMD ["/app/bench-rullst"]
