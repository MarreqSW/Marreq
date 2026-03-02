# Semantic Search (AI) — Setup + Internals

Marreq includes an optional **semantic search** feature for requirements:
- **Hybrid search**: lexical full‑text search + vector similarity search
- **RAG answers**: optional AI answers grounded in your requirements with citations
- **Automatic indexing**: new/updated requirements are queued for embedding generation

All AI processing runs locally using Ollama, so your requirements stay on your machine/network.

---

## Quick Start

### 1) Start PostgreSQL (with pgvector)

Use the repo’s `docker-compose.yml` (it uses a `pgvector/pgvector` Postgres image):

```bash
docker compose up -d db
```

### 2) Initialize the database schema + sample data

This project uses migrations for schema and `init_complete.sql` for sample data:

```bash
./scripts/setup_database.sh
```

Manual equivalent:

```bash
docker exec -i $(docker compose ps -q db) psql -U rust -d postgres -c "CREATE DATABASE marreq;"
DATABASE_URL='postgres://rust:rust@localhost:5432/Marreq' diesel migration run
docker exec -i $(docker compose ps -q db) psql -U rust -d marreq < scripts/init_complete.sql
```

### 3) Install and run Ollama

See `doc/OLLAMA_SETUP.md` for full instructions. Minimal:

```bash
ollama serve
```

### 4) Download models

```bash
ollama pull nomic-embed-text  # embeddings (768 dimensions)
ollama pull llama3.2          # optional RAG answers
```

### 5) Configure `.env`

```bash
# Embeddings (required for semantic/vector search)
EMBEDDINGS_ENABLED=true
EMBEDDING_PROVIDER=ollama
OLLAMA_URL=http://localhost:11434

# RAG answers (optional)
RAG_ENABLED=true
RAG_MODEL=llama3.2
```

### 6) Start Marreq

```bash
cargo run --bin marreq
```

### 7) One-time reindex (for existing requirements)

New/updated requirements are queued automatically, but for projects that already have requirements, trigger a reindex:

```bash
curl -X POST http://localhost:8000/api/projects/1/requirements/reindex \
  -H "Cookie: session=<your-session-cookie>"
```

---

## Feature Overview

### Hybrid retrieval

Marreq combines:
1) **Lexical search** via Postgres full‑text search (`tsvector`)
2) **Vector search** via pgvector cosine similarity over embeddings
3) **Fusion** using Reciprocal Rank Fusion (RRF)

### RAG answers

When enabled, Marreq can generate an answer by:
1) retrieving top‑K relevant requirements, then
2) asking an Ollama LLM to answer using that retrieved context.

Marreq attempts to extract citations like `[REQ-123]` from the answer.

---

## Architecture (High Level)

```
┌─────────────────────────────────────────────────────────────────┐
│                        Frontend (Modal)                         │
│  Query → Filters → Search Button → Results List → RAG Answer    │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      API Endpoints                              │
│  /api/projects/<id>/requirements/semantic_search                │
│  /api/projects/<id>/requirements/ask                            │
│  /api/projects/<id>/requirements/reindex                        │
│  /api/projects/<id>/requirements/index_status                   │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Backend Services                             │
│  - Hybrid retrieval (lexical + vector + RRF)                    │
│  - Optional answer generation                                   │
│  - Index queue processing                                       │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Ollama (Local AI)                            │
│  - Embeddings: /api/embed                                       │
│  - LLM chat:  /api/chat                                         │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      PostgreSQL                                 │
│  - requirement_versions.search_vector (tsvector)                │
│  - requirement_embeddings (pgvector)                            │
│  - embedding_index_queue                                        │
└─────────────────────────────────────────────────────────────────┘
```

---

## Code Map (Backend)

Core modules live under `src/services/semantic_search/`:
- `config.rs`: reads env vars into `SemanticSearchConfig`
- `document_builder.rs`: builds the embedding document + SHA-256 content hash
- `embedding_provider.rs`: embeddings provider abstraction (Ollama + mock)
- `llm_provider.rs`: LLM provider abstraction (Ollama + mock)
- `indexing_service.rs`: writes embeddings and manages `embedding_index_queue`
- `search_service.rs`: lexical search + vector search + RRF + optional `ask()`

API endpoints are in `src/api/semantic_search.rs`.

Automatic background indexing runs via a Rocket fairing in `src/fairings/semantic_index.rs`.

---

## Database Objects (Created by migrations)

### Extension

- `vector` (pgvector)

### Tables

- `requirement_embeddings`
  - stores `embedding vector(768)` plus metadata (`embedding_model`, `content_hash`, timestamps)
  - `requirement_id` is the primary key and CASCADE deletes with the requirement
- `embedding_index_queue`
  - tracks indexing jobs (`pending`, `processing`, `completed`, `failed`)
  - unique per `requirement_id` (only one active job per requirement)
- `requirement_versions.search_vector`
  - `tsvector` column used for lexical full‑text search

### Indexes and triggers

- ANN index on `requirement_embeddings.embedding` using HNSW + cosine distance ops
- GIN index on `requirement_versions.search_vector`
- trigger `requirement_versions_search_vector_trigger` keeps `search_vector` up to date on insert/update

---

## How Search Works

### 1) Lexical search (full‑text search)

The lexical pass ranks requirements using Postgres full‑text search:
- ranks via `ts_rank_cd(search_vector, to_tsquery(...))`
- filters via `search_vector @@ to_tsquery(...)`

Marreq builds an OR‑based `tsquery` string for “question-like” inputs:
- splits words
- drops short words and a small stop-word list
- adds `:*` prefix matching
- joins tokens with ` | ` for recall

### 2) Vector search (pgvector)

The vector pass:
1) generates an embedding for the query via Ollama (`/api/embed`)
2) searches `requirement_embeddings` ordered by cosine distance

### 3) Fusion (RRF)

Marreq combines the two ranked lists using Reciprocal Rank Fusion:
- contribution is `1 / (RRF_K + rank)` with `RRF_K = 60`
- avoids mixing raw lexical/vectors scores directly (no scale matching problems)

### 4) Reference-code shortcut

If the query looks like a reference code (dash + digits), Marreq short-circuits to a direct lookup.

---

## How Indexing Works

### Embedding document

Each requirement is converted into a deterministic embedding document, e.g.:

```
[REF] REQ-SYS-001
[TITLE] System shall process inputs
[DESC] The system shall process all valid inputs within 100ms
[RATIONALE] Required for real-time operation
[CATEGORY] Functional
[STATUS] Draft
```

A SHA‑256 hash derived from (document + model name) is stored as `content_hash` so embeddings are only regenerated when content changes (or the model changes).

### Queue lifecycle

On requirement create/update/import:
- the requirement is upserted into `embedding_index_queue` with status `pending`

Background processing:
- runs periodically
- processes the oldest pending items first
- marks each as `processing`, then `completed` or `failed`

Queue statuses:
- `pending`: waiting
- `processing`: active
- `completed`: indexed successfully
- `failed`: error occurred

---

## Configuration

### Environment variables

| Variable | Description | Default |
|----------|-------------|---------|
| `EMBEDDINGS_ENABLED` | Enable embedding generation | `false` |
| `EMBEDDING_PROVIDER` | Provider (`ollama` or `mock`) | `ollama` |
| `EMBEDDING_MODEL` | Ollama embedding model | `nomic-embed-text` |
| `EMBEDDING_DIM` | Embedding dimension | `768` |
| `OLLAMA_URL` | Ollama server URL | `http://localhost:11434` |
| `RAG_ENABLED` | Enable answer generation | `false` |
| `RAG_MODEL` | Ollama LLM model | `llama3.2` |
| `RAG_MAX_TOKENS` | Max tokens for answer | `1024` |
| `RAG_TOP_K` | Results used as answer context | `10` |

### Model notes

- `nomic-embed-text` uses 768 dimensions, matching the default DB column type.
- If you choose a different embedding model dimension, you must also change the DB column type accordingly (and rebuild existing embeddings).

---

## Usage

### UI

1) Open a project’s Requirements page
2) Click **AI Search** (or press `Ctrl+K` / `Cmd+K`)
3) Enter a query:
   - keywords (“battery endurance”)
   - reference codes (“REQ-PWR-002”)
   - questions (“What are the thermal requirements?”)
4) (Optional) use filters

### API

#### Search requirements

```http
GET /api/projects/{project_id}/requirements/semantic_search?q=<query>&k=<limit>
```

#### Ask a question (answer generation)

```http
POST /api/projects/{project_id}/requirements/ask
Content-Type: application/json

{
  "query": "What are the safety requirements?",
  "k": 10
}
```

#### Reindex a project (admin only)

```http
POST /api/projects/{project_id}/requirements/reindex
```

#### Index status

```http
GET /api/projects/{project_id}/requirements/index_status
```

---

## Troubleshooting

### Ollama server not reachable
- start Ollama: `ollama serve`
- confirm it responds: `curl http://localhost:11434/api/tags`

### Model not found
- pull the model: `ollama pull nomic-embed-text`

### pgvector not available
- use the repo’s docker compose service (recommended), or install pgvector for your Postgres distribution

### Slow performance
- use GPU acceleration for Ollama if available
- use smaller models (e.g. `llama3.2:1b`)

---

## Security Notes

1) Data stays local (Ollama runs on your host)
2) Searches are project-scoped and guarded by existing authorization checks
3) Do not expose Ollama to the public internet

---

## Development

### Tests

```bash
cargo test semantic_search --lib
```

### Mock provider (no Ollama required)

```bash
EMBEDDINGS_ENABLED=true
EMBEDDING_PROVIDER=mock cargo test
```
