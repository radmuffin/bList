# Multi-stage Docker build with cargo-chef dependency caching
FROM lukemathwalker/cargo-chef:latest-rust-1-alpine AS chef
WORKDIR /app
RUN apk add --no-cache musl-dev build-base

FROM chef AS planner
COPY Cargo.toml Cargo.lock ./
COPY src ./src
RUN cargo chef prepare --recipe-path recipe.json

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
# Build dependencies - this layer is cached unless Cargo.lock/Cargo.toml changes
RUN cargo chef cook --release --recipe-path recipe.json
# Build application code
COPY Cargo.toml Cargo.lock ./
COPY src ./src
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
