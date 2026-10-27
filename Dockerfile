# Stage 1: Build release binary with latest Rust compiler
FROM rust:latest as builder

WORKDIR /usr/src/entropiq

# Install build dependencies
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates pkg-config && rm -rf /var/lib/apt/lists/*

# Copy workspace sources and deterministic lockfile
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates

# Compile supervisor binary in release mode
RUN cargo build --release --bin supervisor -p alpaca-streamer

# Stage 2: Ultra-light runtime container (< 30 MB)
FROM debian:bookworm-slim

# Install root TLS certificates for WebSocket and HTTPS
RUN apt-get update && apt-get install -y --no-install-recommends ca-certificates curl && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary from builder
COPY --from=builder /usr/src/entropiq/target/release/supervisor /app/supervisor

# Expose Render HTTP port
ENV PORT=10000
EXPOSE 10000

# Native health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:${PORT}/health || exit 1

CMD ["/app/supervisor"]
