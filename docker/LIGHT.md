# Lightweight Docker deployment

`docker-compose.light.yml` is the resource-conscious Marreq stack for hosts that do not use semantic/vector search or RAG.

Unlike `docker-compose.yml`, the light stack does **not** define an Ollama service, does not depend on Ollama, and forces both `EMBEDDINGS_ENABLED` and `RAG_ENABLED` to `false`. Docker therefore does not pull or start the `ollama/ollama` image.

The PostgreSQL image remains `pgvector/pgvector` because the Marreq database schema and migrations still include vector-related database support even when semantic features are disabled.

## Local / HTTP usage

From the repository root:

```bash
docker compose -f docker/docker-compose.light.yml up -d --build
```

The self-hosted SPA is available on `http://127.0.0.1:8080` by default.

To start only the runtime services needed by the self-hosted application:

```bash
docker compose \
  -f docker/docker-compose.light.yml \
  up -d --build db marreq-server frontend
```

## Production / HTTPS usage

The light stack can be combined directly with the production hardening override:

```bash
docker compose \
  -f docker/docker-compose.light.yml \
  -f docker/docker-compose.prod.yml \
  config >/dev/null

docker compose \
  -f docker/docker-compose.light.yml \
  -f docker/docker-compose.prod.yml \
  up -d --build db marreq-server frontend
```

Set the production variables described in `.env.example`, including a real `ROCKET_SECRET_KEY`, `MARREQ_PUBLIC_BASE_URL`, and `CSRF_ALLOWED_ORIGINS`. The production override enables secure session cookies and is intended to run behind the external TLS reverse proxy described in `docker/README.md`.

## Cloud variant

The light configuration also contains the `cloud` profile without Ollama:

```bash
docker compose \
  -f docker/docker-compose.light.yml \
  --profile cloud \
  up -d --build db marreq-cloud frontend-cloud
```

For production, add `-f docker/docker-compose.prod.yml` after the light file.

## Full stack vs light stack

Use `docker-compose.yml` when semantic search/RAG backed by Ollama is required. Use `docker-compose.light.yml` when those features are intentionally disabled, especially on memory-constrained hosts.

Do not combine `docker-compose.yml` and `docker-compose.light.yml`; they are alternative base stacks. `docker-compose.prod.yml` is an override and can be combined with either base stack.
