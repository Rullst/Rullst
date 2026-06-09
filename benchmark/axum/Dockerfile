FROM rust:1.95-slim-bookworm AS builder
WORKDIR /usr/src/axum
COPY Cargo.toml ./
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim
WORKDIR /app
COPY --from=builder /usr/src/axum/target/release/bench-axum /app/bench-axum
EXPOSE 3000
CMD ["/app/bench-axum"]
