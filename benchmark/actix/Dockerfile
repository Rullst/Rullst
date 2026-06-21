FROM rust:1.85-bookworm AS builder
WORKDIR /usr/src/app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y libpq-dev && rm -rf /var/lib/apt/lists/*
WORKDIR /app
COPY --from=builder /usr/src/app/target/release/actix-bench /usr/local/bin/actix-bench
ENV DATABASE_URL=postgres://postgres:postgres@db:5432/benchdb
EXPOSE 8000
CMD ["actix-bench"]
