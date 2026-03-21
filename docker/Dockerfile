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

# Workspace root + backend crate (cached unless manifests change)
COPY Cargo.toml Cargo.lock ./
COPY backend/Cargo.toml backend/diesel.toml ./backend/

# Create dummy package source to cache dependencies
RUN mkdir -p backend/src && echo "fn main() {}" > backend/src/main.rs

# Build dependencies only (workspace resolves from root)
RUN cargo build --release -p marreq 2>/dev/null || true

# Install diesel_cli for migrations (postgres only)
RUN cargo install diesel_cli --no-default-features --features postgres

# Remove dummy and copy real backend source
RUN rm -rf backend/src
COPY backend/src ./backend/src
COPY backend/migrations ./backend/migrations
COPY backend/scripts ./backend/scripts

# Build the application (overwrite the dummy binary)
RUN touch backend/src/main.rs && cargo build --release -p marreq

# Runtime stage
FROM debian:bookworm-slim

RUN apt-get update && apt-get install -y --no-install-recommends \
    libpq5 \
    postgresql-client \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy binary and migrations from builder
COPY --from=builder /app/target/release/marreq /app/marreq
COPY --from=builder /app/backend/migrations /app/migrations
COPY --from=builder /app/backend/scripts /app/scripts
COPY --from=builder /usr/local/cargo/bin/diesel /usr/local/bin/diesel

# Rocket.toml at /app (cwd for entrypoint). template_dir defaults to templates/ relative to this file → /app/templates
COPY backend/Rocket.toml ./

COPY --from=builder /app/backend/src/html/static /app/backend/src/html/static
COPY backend/templates /app/templates

# diesel CLI uses /app/diesel.toml + /app/migrations (see docker-entrypoint.sh)
COPY backend/diesel.toml ./diesel.toml

EXPOSE 8000

# Wait for DB, run migrations, then start the app
COPY docker/docker-entrypoint.sh /docker-entrypoint.sh
RUN chmod +x /docker-entrypoint.sh
ENTRYPOINT ["/docker-entrypoint.sh"]
CMD ["/app/marreq"]
