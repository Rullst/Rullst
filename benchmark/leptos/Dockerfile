FROM rust:1.95-slim AS builder
WORKDIR /usr/src/leptos
COPY Cargo.toml ./
COPY src ./src
RUN cargo build --release

FROM debian:bookworm-slim
WORKDIR /app
COPY --from=builder /usr/src/leptos/target/release/bench-leptos /app/bench-leptos
EXPOSE 3000
CMD ["/app/bench-leptos"]
