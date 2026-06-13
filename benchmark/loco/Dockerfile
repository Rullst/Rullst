FROM rust:1.85-slim-bookworm AS builder
WORKDIR /usr/src/loco
COPY Cargo.toml ./
COPY src ./src
COPY config ./config
RUN cargo build --release

FROM debian:bookworm-slim
WORKDIR /app
COPY --from=builder /usr/src/loco/target/release/bench-loco /app/bench-loco
COPY --from=builder /usr/src/loco/config /app/config
EXPOSE 3000
CMD ["/app/bench-loco"]
