# Build stage
FROM rust:1-trixie AS builder

# C toolchain for native crates (aws-lc-sys via rustls, openssl-sys, pq-sys)
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    pkg-config \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Workspace manifests (cached unless manifests change)
COPY Cargo.toml Cargo.lock ./
COPY marreq-core/Cargo.toml marreq-core/diesel.toml ./marreq-core/
COPY marreq-server/Cargo.toml ./marreq-server/
COPY marreq-cloud/Cargo.toml ./marreq-cloud/
COPY tools/marreq-contracts/Cargo.toml ./tools/marreq-contracts/
COPY tools/marreq-migrations/Cargo.toml ./tools/marreq-migrations/

# Cargo resolves the full locked workspace. These targets satisfy discovery
# and fail compilation if an inactive package becomes a build dependency.
RUN mkdir -p marreq-core/src marreq-server/src marreq-cloud/src tools/marreq-contracts/src tools/marreq-migrations/src && \
    for target in marreq-server/src/main.rs marreq-cloud/src/main.rs marreq-cloud/src/lib.rs \
        marreq-core/src/lib.rs tools/marreq-contracts/src/lib.rs tools/marreq-contracts/src/main.rs \
        tools/marreq-migrations/src/lib.rs tools/marreq-migrations/src/main.rs; do \
        echo 'compile_error!("workspace placeholder must not be compiled");' > "$target"; \
    done && \
    cargo fetch --locked

ARG MARREQ_BIN=marreq-server
RUN case "$MARREQ_BIN" in marreq-server|marreq-cloud) ;; *) exit 1 ;; esac && \
    rm -rf marreq-core/src "${MARREQ_BIN}/src" tools/marreq-migrations/src

COPY marreq-core/src ./marreq-core/src
COPY marreq-core/frontend_compatibility.json ./marreq-core/frontend_compatibility.json
COPY marreq-core/migrations ./marreq-core/migrations
COPY marreq-core/scripts ./marreq-core/scripts
COPY ${MARREQ_BIN}/src ./${MARREQ_BIN}/src
COPY tools/marreq-migrations/src ./tools/marreq-migrations/src
COPY tools/marreq-migrations/build.rs ./tools/marreq-migrations/build.rs

# Build the selected binary (MARREQ_VERSION / MARREQ_GIT_SHA baked via option_env!)
ARG MARREQ_VERSION=
ARG MARREQ_GIT_SHA=unknown
ENV MARREQ_GIT_SHA=${MARREQ_GIT_SHA}
ENV MARREQ_VERSION=${MARREQ_VERSION}
RUN cargo build --locked --release -p "${MARREQ_BIN}" -p marreq-migrations -p marreq-core \
    --bin "${MARREQ_BIN}" --bin marreq-migrations --bin marreq-backup

# Runtime stage
FROM debian:trixie-slim

ARG MARREQ_BIN=marreq-server
ARG MARREQ_VERSION=
ARG MARREQ_GIT_SHA=unknown
LABEL org.opencontainers.image.version="${MARREQ_VERSION:-dev}" \
      org.opencontainers.image.revision="${MARREQ_GIT_SHA}"

# trixie's postgresql-client is 17, matching the pg17 database image, so the
# in-app backup (pg_dump) writes dumps that the server can restore.
RUN apt-get update && apt-get install -y --no-install-recommends \
    libpq5 \
    postgresql-client \
    ca-certificates \
    curl \
    && rm -rf /var/lib/apt/lists/*

# Application and one-shot operational commands share an unprivileged UID/GID.
RUN groupadd --system --gid 10001 marreq \
    && useradd  --system --uid 10001 --gid marreq --home-dir /app --shell /usr/sbin/nologin marreq

WORKDIR /app

# Copy the backend, migration and backup binaries, plus runtime resources
COPY --from=builder /app/target/release/${MARREQ_BIN} /usr/local/bin/marreq
COPY --from=builder /app/marreq-core/migrations /app/migrations
COPY --from=builder /app/marreq-core/scripts /app/scripts
COPY --from=builder /app/marreq-core/src/schema.patch /app/src/schema.patch
COPY --from=builder /app/target/release/marreq-migrations /usr/local/bin/marreq-migrations
COPY --from=builder /app/target/release/marreq-backup /usr/local/bin/marreq-backup

# Rocket.toml is crate-specific; pull from the selected binary's crate
COPY ${MARREQ_BIN}/Rocket.toml ./Rocket.toml

# Preserve the Diesel configuration alongside the migration files
COPY marreq-core/diesel.toml ./diesel.toml

# Attachment storage (MARREQ_ATTACHMENTS_DIR). Created here so a new named
# volume mounted on it inherits the marreq owner.
RUN mkdir -p /var/lib/marreq/attachments \
    && chown -R marreq:marreq /app /var/lib/marreq
ENV MARREQ_ATTACHMENTS_DIR=/var/lib/marreq/attachments

EXPOSE 8000

# Start the app; the binary performs a read-only schema preflight
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
