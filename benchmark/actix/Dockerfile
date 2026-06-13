FROM rust:1.85-slim-bookworm AS builder
WORKDIR /usr/src/actix
COPY Cargo.toml ./
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim
WORKDIR /app
COPY --from=builder /usr/src/actix/target/release/bench-actix /app/bench-actix
EXPOSE 3000
CMD ["/app/bench-actix"]
