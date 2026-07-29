FROM rust:bookworm AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config libssl-dev clang lld \
    && rm -rf /var/lib/apt/lists/*

COPY rust-toolchain.toml ./

# Copy the full workspace (root Cargo.toml declares members = ["crates/*"],
# so every member's manifest must be present for `cargo build` to parse the workspace)
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Build release binary
RUN cargo build --release --package mage-worker

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/mage-worker /app/mage-worker

CMD ["/app/mage-worker"]
