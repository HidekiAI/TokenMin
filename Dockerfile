# Use the latest Rust stable image
FROM rust:1-slim-bookworm AS builder

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libsqlite3-dev \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/tokenmin

# Step 1: Pre-build dependencies for better caching
COPY Cargo.toml Cargo.lock ./
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Step 2: Copy source and build application
COPY src ./src
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y \
    libsqlite3-0 \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from=builder /usr/src/tokenmin/target/release/TokenMin /app/tokenmin

# Default environment variables
ENV TOKENMIN_DB=/tmp/tokenmin.db
# NOTE: OLLAMA_URL is intentionally not set by default.
# Set OLLAMA_URL at runtime (e.g. via -e OLLAMA_URL=http://...).
# On Linux, to reach a host-side Ollama, run the container with:
#   --add-host=host.docker.internal:host-gateway
# and then set: -e OLLAMA_URL=http://host.docker.internal:11434

# The container will run the watcher
CMD ["./tokenmin"]
