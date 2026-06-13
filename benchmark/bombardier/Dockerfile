FROM golang:1.22-alpine AS builder
RUN apk add --no-cache git
WORKDIR /go/src/github.com/codesenberg/bombardier
RUN git clone --depth 1 --branch v2.0.3 https://github.com/codesenberg/bombardier.git .
RUN go build -o /go/bin/bombardier .

FROM alpine:3.18
COPY --from=builder /go/bin/bombardier /usr/local/bin/bombardier
ENTRYPOINT ["bombardier"]
