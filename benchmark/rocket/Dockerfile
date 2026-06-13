FROM rust:1.85-slim-bookworm AS builder
WORKDIR /usr/src/rocket
COPY Cargo.toml ./
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim
WORKDIR /app
COPY --from=builder /usr/src/rocket/target/release/bench-rocket /app/bench-rocket
EXPOSE 8000
ENV ROCKET_PROFILE=release
CMD ["/app/bench-rocket"]
