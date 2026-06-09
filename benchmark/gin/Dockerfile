FROM golang:1.22-alpine AS builder
WORKDIR /app
COPY go.mod main.go ./
RUN go mod tidy && CGO_ENABLED=0 GOOS=linux go build -o bench-gin .

FROM alpine:3.18
WORKDIR /app
COPY --from=builder /app/bench-gin /app/bench-gin
EXPOSE 3000
CMD ["/app/bench-gin"]
