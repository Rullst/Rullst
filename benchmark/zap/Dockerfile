FROM alpine:3.19 AS builder
RUN apk add --no-cache curl tar xz git clang gcc make musl-dev
RUN curl -L https://ziglang.org/download/0.13.0/zig-linux-x86_64-0.13.0.tar.xz | tar -xJ --strip-components=1 -C /usr/local/bin
WORKDIR /app
COPY build.zig build.zig.zon ./
RUN zig fetch --save=zap https://github.com/zigzap/zap/archive/refs/tags/v0.8.0.tar.gz
COPY src ./src
RUN zig build -Doptimize=ReleaseFast

FROM alpine:3.19
WORKDIR /app
COPY --from=builder /app/zig-out/bin/bench-zap /app/bench-zap
EXPOSE 3000
CMD ["/app/bench-zap"]
