# Multi-stage build for ReqMan Rust application
FROM rust:1.90-bookworm as builder

# Install system dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    libpq-dev \
    build-essential \
    libclang-dev \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy manifests first for better caching
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to build dependencies
RUN mkdir src && echo "fn main() {println!(\"dummy\");}" > src/main.rs

# Build dependencies (this layer will be cached if Cargo.toml doesn't change)
RUN cargo build --release
# Remove the dummy artifacts to force rebuild of main binary
RUN rm -rf target/release/.fingerprint/req_man-*
RUN rm -f target/release/req_man target/release/req_man.d
RUN rm -f target/release/deps/req_man-*
RUN rm -rf src

# Copy source code
COPY src ./src
COPY templates ./templates
COPY migrations ./migrations
COPY Rocket.toml ./
COPY diesel.toml ./

# Build the application
RUN cargo build --release

# Runtime stage
FROM debian:bookworm

# Install runtime dependencies and debugging tools
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libpq5 \
    curl \
    strace \
    procps \
    net-tools \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN groupadd -r reqman && useradd -r -g reqman reqman

# Create app directory
WORKDIR /app

# Copy the binary from builder stage
COPY --from=builder /app/target/release/req_man /app/req_man

# Copy templates, migrations, and static files
COPY --from=builder /app/templates ./templates
COPY --from=builder /app/migrations ./migrations
COPY --from=builder /app/src/html/static ./src/html/static
COPY --from=builder /app/diesel.toml ./

# Copy Docker-specific Rocket configuration
COPY Rocket.docker.toml ./Rocket.toml

# Change ownership to non-root user
RUN chown -R reqman:reqman /app

# Switch to non-root user
USER reqman

# Expose port
EXPOSE 8000

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD curl -f http://localhost:8000/ || exit 1

# Run the application
CMD ["./req_man"]
