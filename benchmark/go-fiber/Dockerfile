FROM golang:1.25-alpine AS builder

WORKDIR /app

COPY go.mod go.sum ./
RUN go mod download

COPY . .

# Build with optimization
RUN CGO_ENABLED=0 GOOS=linux go build -ldflags="-w -s" -o benchmark-go-fiber main.go

FROM alpine:latest

WORKDIR /app
COPY --from=builder /app/benchmark-go-fiber .

EXPOSE 8080

CMD ["./benchmark-go-fiber"]
