# Build stage
FROM rust:alpine AS builder

WORKDIR /app

# Install build dependencies
RUN apk add --no-cache musl-dev pkgconfig openssl-dev

COPY rust-toolchain.toml ./

# Copy the full workspace (root Cargo.toml declares members = ["crates/*"],
# so every member's manifest must be present for `cargo build` to parse the workspace)
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Build release binary
RUN cargo build --release --package mage-broker

# Runtime stage
FROM alpine:latest

WORKDIR /app

# Install runtime dependencies
RUN apk add --no-cache ca-certificates libssl3

# Copy binary from builder
COPY --from=builder /app/target/release/mage-broker /app/mage-broker

CMD ["/app/mage-broker"]
