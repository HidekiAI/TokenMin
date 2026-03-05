# Use the latest Rust stable image
FROM rust:1-slim-bookworm AS builder

# Install system dependencies
RUN apt-get update && apt-get install -y 
    pkg-config 
    libsqlite3-dev 
    curl 
    unzip 
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/src/tokenmin
COPY . .

# Build the application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm-slim
RUN apt-get update && apt-get install -y 
    libsqlite3-0 
    curl 
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app
COPY --from:builder /usr/src/tokenmin/target/release/TokenMin /app/tokenmin
COPY --from:builder /usr/src/tokenmin/scripts /app/scripts

# Default environment variables
ENV TOKENMIN_DB=/tmp/tokenmin.db
ENV OLLAMA_URL=http://host.docker.internal:11434

# The container will run the watcher
CMD ["./tokenmin"]
