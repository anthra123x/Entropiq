# Build stage: compile static release binary
FROM rust:1.80-slim as builder

WORKDIR /app
COPY . .

RUN cargo build --release --bin supervisor -p alpaca-streamer

# Runtime stage: ultra-lightweight distroless/debian
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y ca-certificates && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /app/target/release/supervisor /app/entropiq-supervisor

ENV RUST_LOG=info
CMD ["/app/entropiq-supervisor"]
