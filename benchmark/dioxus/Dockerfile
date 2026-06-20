FROM rust:1.85-bookworm AS builder

WORKDIR /app
# We create a dummy cargo project to cache dependencies
RUN USER=root cargo new --bin dioxus_bench
WORKDIR /app/dioxus_bench
COPY benchmark/dioxus/Cargo.toml benchmark/dioxus/Cargo.lock* ./
RUN cargo build --release --bin dioxus_bench || true

# Now copy the source code
COPY benchmark/dioxus/src ./src
# Build the actual application
RUN cargo build --release --bin dioxus_bench

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libpq-dev && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /app/dioxus_bench/target/release/dioxus_bench .
ENV PORT=3000
EXPOSE 3000
CMD ["./dioxus_bench"]
