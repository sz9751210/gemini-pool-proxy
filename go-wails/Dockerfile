FROM golang:1.21-alpine AS builder

WORKDIR /src

COPY go.mod go.sum ./
RUN go mod download

COPY . .
RUN CGO_ENABLED=0 GOOS=linux GOARCH=amd64 go build -trimpath -ldflags="-s -w" -o /out/go-wails-server ./cmd/server

FROM alpine:3.20

RUN addgroup -S app && adduser -S -G app app

COPY --from=builder /out/go-wails-server /usr/local/bin/go-wails-server

USER app
WORKDIR /app

EXPOSE 18080

ENV RUNTIME_BIND_HOST=0.0.0.0
ENV RUNTIME_PORT_START=18080
ENV RUNTIME_PORT_END=18080

ENTRYPOINT ["/usr/local/bin/go-wails-server"]
