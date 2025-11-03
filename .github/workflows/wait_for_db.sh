#!/bin/bash
set -euo pipefail

echo "Waiting for PostgreSQL container to stabilize..."
sleep 5

# Get container ID
DB_CONTAINER=$(docker compose ps -q db)

# Wait for container to be healthy
for i in {1..30}; do
  echo "Attempt $i/30: Checking database health..."
  
  # Check if container is running
  if ! docker ps -q --filter "id=$DB_CONTAINER" | grep -q .; then
    echo "Container stopped unexpectedly, showing logs:"
    docker compose logs db
    exit 1
  fi
  
  # Check health status
  HEALTH=$(docker inspect --format='{{.State.Health.Status}}' "$DB_CONTAINER" 2>/dev/null || echo "unknown")
  echo "Health status: $HEALTH"
  
  if [ "$HEALTH" = "healthy" ]; then
    echo "PostgreSQL is healthy and ready!"
    exit 0
  elif [ "$HEALTH" = "starting" ]; then
    echo "PostgreSQL is starting..."
  elif [ "$HEALTH" = "unhealthy" ]; then
    echo "PostgreSQL is unhealthy, showing logs:"
    docker compose logs db --tail 50
    exit 1
  else
    # No healthcheck yet, try pg_isready directly
    if docker compose exec -T db pg_isready -U rust >/dev/null 2>&1; then
      echo "PostgreSQL is ready (via pg_isready)!"
      exit 0
    fi
  fi
  
  sleep 2
done

echo "Failed to start database after 30 attempts"
docker compose logs db
exit 1
