# Docker Layout and Usage

All container-related files live in this directory.

## Files

- `docker-compose.yml`: Primary local stack (`db`, `ollama`, `marreq`, `adminer`)
- `docker-compose.ci.yml`: CI-specific compose overrides
- `Dockerfile`: Application image build definition
- `Dockerfile.dockerignore`: Build context exclusions for `Dockerfile`
- `docker-entrypoint.sh`: Container startup script (wait for DB + run migrations + start app)

## Common Commands (from repo root)

Start only the database:

```bash
docker compose -f docker/docker-compose.yml up -d db
```

Start the full stack:

```bash
docker compose -f docker/docker-compose.yml up -d
```

View logs:

```bash
docker compose -f docker/docker-compose.yml logs -f
```

Stop the stack:

```bash
docker compose -f docker/docker-compose.yml down
```

## Build the app image directly

```bash
docker build -f docker/Dockerfile -t marreq:local .
```

## CI Compose Overrides

When you need CI-like behavior locally, combine both files:

```bash
docker compose \
  -f docker/docker-compose.yml \
  -f docker/docker-compose.ci.yml \
  up -d db
```

## Script Compatibility

The DB helper scripts in `scripts/` already use `docker/docker-compose.yml` internally, so existing commands like `./scripts/db_setup.sh` keep working.

## Troubleshooting

### Database Connection Issues

```bash
# Check if database container is running
docker compose -f docker/docker-compose.yml ps db

# Check database connectivity
docker compose -f docker/docker-compose.yml exec -T db psql -U rust -d marreq -c "SELECT 1;"

# Restart database container
docker compose -f docker/docker-compose.yml restart db
```

### Database Reset

```bash
# Complete database reset
docker compose -f docker/docker-compose.yml exec -T db psql -U rust -d postgres -c "DROP DATABASE IF EXISTS marreq;"
./scripts/db_setup.sh --seed
```

### Verification Commands

```bash
# Verify database setup
docker compose -f docker/docker-compose.yml exec -T db psql -U rust -d marreq -c "\dt"

# Check user creation
docker compose -f docker/docker-compose.yml exec -T db psql -U rust -d marreq -c "SELECT username, name, is_admin FROM users;"

# Verify sample data
docker compose -f docker/docker-compose.yml exec -T db psql -U rust -d marreq -c "SELECT COUNT(*) as requirements FROM requirements;"
```
