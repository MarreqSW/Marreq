# Build stage
FROM rust:1-bookworm AS builder

ARG MARREQ_BIN=marreq-server

# Native deps for libxlsxwriter-sys (build from source) and bindgen (libclang)
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    cmake \
    libclang-dev \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Workspace manifests (cached unless manifests change)
COPY Cargo.toml Cargo.lock ./
COPY marreq-core/Cargo.toml marreq-core/diesel.toml ./marreq-core/
COPY marreq-server/Cargo.toml ./marreq-server/
COPY marreq-cloud/Cargo.toml ./marreq-cloud/

# Dummy sources so cargo can resolve and cache dependencies
RUN mkdir -p marreq-core/src marreq-server/src marreq-cloud/src && \
    echo "fn main() {}" > marreq-server/src/main.rs && \
    echo "fn main() {}" > marreq-cloud/src/main.rs && \
    echo "" > marreq-core/src/lib.rs

# Cache dependencies only (ignore build errors from empty stubs)
RUN cargo build --release -p ${MARREQ_BIN} 2>/dev/null || true

# Install diesel_cli for migrations (postgres only)
RUN cargo install diesel_cli --no-default-features --features postgres

# Copy real sources
RUN rm -rf marreq-core/src marreq-server/src marreq-cloud/src
COPY marreq-core/src ./marreq-core/src
COPY marreq-core/migrations ./marreq-core/migrations
COPY marreq-core/scripts ./marreq-core/scripts
COPY marreq-server/src ./marreq-server/src
COPY marreq-cloud/src ./marreq-cloud/src

# Build the selected binary
RUN touch marreq-core/src/lib.rs marreq-server/src/main.rs marreq-cloud/src/main.rs && \
    cargo build --release -p ${MARREQ_BIN}

# Runtime stage
FROM debian:bookworm-slim

ARG MARREQ_BIN=marreq-server

RUN apt-get update && apt-get install -y --no-install-recommends \
    libpq5 \
    postgresql-client \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Run the application as an unprivileged user.  The migration step in
# docker-entrypoint.sh only needs DB access, so dropping privileges before
# Rocket binds is safe.  UID/GID are stable so bind-mounted volumes from
# docker-compose stay readable.
RUN groupadd --system --gid 10001 marreq \
    && useradd  --system --uid 10001 --gid marreq --home-dir /app --shell /usr/sbin/nologin marreq

WORKDIR /app

# Copy binary, migrations, scripts and diesel CLI from builder
COPY --from=builder /app/target/release/${MARREQ_BIN} /usr/local/bin/marreq
COPY --from=builder /app/marreq-core/migrations /app/migrations
COPY --from=builder /app/marreq-core/scripts /app/scripts
COPY --from=builder /usr/local/cargo/bin/diesel /usr/local/bin/diesel

# Rocket.toml is crate-specific; pull from the selected binary's crate
COPY ${MARREQ_BIN}/Rocket.toml ./Rocket.toml

# diesel CLI uses /app/diesel.toml + /app/migrations (see docker-entrypoint.sh)
COPY marreq-core/diesel.toml ./diesel.toml

RUN chown -R marreq:marreq /app

EXPOSE 8000

# Wait for DB, run migrations, then start the app
COPY docker/docker-entrypoint.sh /docker-entrypoint.sh
RUN chmod +x /docker-entrypoint.sh

USER marreq

# Liveness check used by Docker / Compose / Kubernetes.  /api/meta/health does
# not touch the DB or cache so it stays green during transient backend issues
# (use /api/cache/health for a readiness-style probe).
HEALTHCHECK --interval=30s --timeout=5s --start-period=20s --retries=3 \
    CMD curl --fail --silent --show-error http://127.0.0.1:8000/api/meta/health || exit 1

ENTRYPOINT ["/docker-entrypoint.sh"]
CMD ["/usr/local/bin/marreq"]
