FROM rust:1.85-slim-bookworm as builder

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY benches ./benches
COPY templates ./templates

RUN cargo build --release --bin poem-bench

FROM debian:bookworm-slim
WORKDIR /app
COPY --from=builder /app/target/release/poem-bench /app/poem-bench
ENV RUST_LOG=error
EXPOSE 3000
CMD ["/app/poem-bench"]
