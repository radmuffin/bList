# Multi-stage Docker build for lightweight production deployment
FROM rust:1-alpine as builder

RUN apk add --no-cache musl-dev build-base

WORKDIR /app
COPY Cargo.toml Cargo.lock ./
COPY src ./src
COPY static ./static

RUN cargo build --release

FROM alpine:latest

RUN apk add --no-cache ca-certificates

WORKDIR /app
COPY --from=builder /app/target/release/map-bucket-list /app/map-bucket-list
COPY static /app/static

ENV PORT=3000
ENV DATABASE_PATH=/data/pins.db
VOLUME ["/data"]

EXPOSE 3000

CMD ["/app/map-bucket-list"]
