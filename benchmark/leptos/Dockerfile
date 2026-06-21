FROM rust:1.85-bookworm AS builder

WORKDIR /app
# We create a dummy cargo project to cache dependencies
RUN USER=root cargo new --bin leptos_bench
WORKDIR /app/leptos_bench
COPY benchmark/leptos/Cargo.toml benchmark/leptos/Cargo.lock* ./
RUN cargo build --release --bin leptos_bench || true

# Now copy the source code
COPY benchmark/leptos/src ./src
# Build the actual application
RUN cargo build --release --bin leptos_bench

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libpq-dev && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/leptos_bench/target/release/leptos_bench .
ENV PORT=3000
EXPOSE 3000
CMD ["./leptos_bench"]
