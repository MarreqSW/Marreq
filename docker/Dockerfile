# Build stage
FROM rust:1-bookworm AS builder

# Native deps for libxlsxwriter-sys (build from source) and bindgen (libclang)
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    cmake \
    libclang-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy manifests and install dependencies (cached unless Cargo.toml/Cargo.lock change)
COPY Cargo.toml Cargo.lock ./
COPY diesel.toml ./

# Create a dummy main to cache dependencies
RUN mkdir src && echo "fn main() {}" > src/main.rs

# Build dependencies only
RUN cargo build --release 2>/dev/null || true

# Install diesel_cli for migrations (postgres only)
RUN cargo install diesel_cli --no-default-features --features postgres

# Remove dummy and copy real source
RUN rm -rf src
COPY src ./src
COPY migrations ./migrations
COPY scripts  ./scripts

# Build the application (overwrite the dummy binary)
RUN touch src/main.rs && cargo build --release

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    libpq5 \
    postgresql-client \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary and migrations from builder
COPY --from=builder /app/target/release/marreq /app/marreq
COPY --from=builder /app/migrations /app/migrations 
COPY --from=builder /app/scripts /app/scripts
COPY --from=builder /usr/local/cargo/bin/diesel /usr/local/bin/diesel

# Copy Rocket.toml for secret_key etc. (database url overridden by env in compose)
COPY Rocket.toml ./

# Static assets and templates (app serves from relative path src/html/static and templates/)
COPY --from=builder /app/src/html/static /app/src/html/static
COPY templates /app/templates

EXPOSE 8000

# Wait for DB, run migrations, then start the app
COPY docker/docker-entrypoint.sh /docker-entrypoint.sh
RUN chmod +x /docker-entrypoint.sh
ENTRYPOINT ["/docker-entrypoint.sh"]
CMD ["/app/marreq"]
