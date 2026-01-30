# ReqMan — Semantic Search / RAG Implementation Report (branch `rag-search`)

This report explains how ReqMan’s “Semantic Search” feature is implemented in the `rag-search` branch (compared to the repo default branch `main`), what “RAG” means, which models are used, how configuration works, and concrete improvement/optimization opportunities.

Related “how to run it” docs already exist:
- `doc/SEMANTIC_SEARCH.md` (user-facing setup + overview)
- `doc/OLLAMA_SETUP.md` (Ollama installation + model pulls)

This document is intentionally more “code + architecture” focused.

---

## 1) What changed vs “master”?

The repository does not have a `master` branch; the default remote branch is `origin/main`.

Compared to `origin/main`, `rag-search` adds ~55 files worth of changes, centered around:

- **Database**: `pgvector` extension + vector storage + full-text search triggers + a background indexing queue
  - `migrations/2026-01-29-000001_add_semantic_search/*`
  - `migrations/2026-01-30-144348-0000_fix_embedding_dimensions/*`
- **Backend**: a new semantic search service module + Rocket API endpoints + a Rocket fairing to process embedding jobs
  - `src/services/semantic_search/*`
  - `src/api/semantic_search.rs`
  - `src/fairings/semantic_index.rs`
- **Frontend**: a modal UI (“AI Search”) on the Requirements page + JS module that calls the new API
  - `templates/requirements/requirements.html.hbs`
  - `templates/requirements/_view_controls.html.hbs`
  - `src/html/static/js/pages/semanticSearch.js`
- **Tooling**: scripts for setup and reindexing
  - `scripts/lazy_setup.sh`
  - `scripts/reindex_project.sh`
  - `scripts/setup_database.sh`

Notable commits (relative to `origin/main`) include:
- `5dbcd05`: initial semantic search modal + API integration
- `871206f`: automatic indexing + configuration
- `aeb3c2c`: improved natural-language lexical query handling

---

## 2) Feature definition: what “Semantic Search” does

ReqMan’s semantic search feature is **hybrid retrieval** + optional **RAG answer generation**:

1. **Hybrid search** returns a ranked list of matching requirements by combining:
   - **Lexical** search (Postgres full-text search / `tsvector`)
   - **Vector** search (dense embeddings + cosine similarity via `pgvector`)
   - Results are fused with **Reciprocal Rank Fusion (RRF)**.

2. **RAG answers** (“Ask a question”) optionally generates a natural-language answer grounded in the retrieved requirements and attempts to produce citations like `[REQ-001]`.

3. **Automatic indexing**:
   - On requirement create/update, the requirement is queued for embedding generation.
   - A Rocket fairing processes the queue periodically in the background.

---

## 3) What is RAG?

**RAG = Retrieval-Augmented Generation.**

Instead of asking an LLM to answer using its internal memory (which can hallucinate), you:

1) **Retrieve** relevant documents/records (here: requirements) from your system, and
2) **Augment** the prompt with that retrieved context, and
3) **Generate** an answer constrained to (and ideally citing) the provided sources.

In ReqMan, the “documents” are individual requirements (title/description/etc.), the retriever is the hybrid search (lexical + vector), and the generator is an Ollama-hosted LLM.

---

## 4) High-level architecture (backend)

### 4.1 Modules and responsibilities

`src/services/semantic_search/*` provides the core:

- `config.rs`
  - Reads env vars once (global `OnceLock`) into `SemanticSearchConfig`.
  - Default models:
    - Embeddings: `nomic-embed-text`
    - RAG LLM: `llama3.2`
- `document_builder.rs`
  - Builds a deterministic “embedding document” from a `DecoratedRequirement`.
  - Computes a SHA-256 hash (`content_hash`) from the document + model name for change detection.
- `embedding_provider.rs`
  - `EmbeddingProvider` trait + implementations:
    - `OllamaEmbeddingProvider` (calls `POST {OLLAMA_URL}/api/embed`)
    - `MockEmbeddingProvider` (deterministic vectors for tests)
- `indexing_service.rs`
  - Writes embeddings into Postgres (`requirement_embeddings`) and manages `embedding_index_queue`.
  - Supports:
    - Queueing individual requirements (`queue_for_indexing`)
    - Processing queue in batches (`process_queue`)
    - Full reindex per project (`reindex_project`)
- `search_service.rs`
  - Hybrid search:
    - `lexical_search()` via Postgres `tsvector` + `to_tsquery`
    - `vector_search()` via `pgvector` cosine distance
    - Combines with RRF
  - Optional RAG:
    - `ask()` calls `search()`, then uses `llm_provider.rs` to generate an answer.
- `llm_provider.rs`
  - `LlmProvider` trait + implementations:
    - `OllamaLlmProvider` (calls `POST {OLLAMA_URL}/api/chat`)
    - `MockLlmProvider` (for tests)
  - Prompt builders and citation extraction (`[REF-CODE]` regex)

### 4.2 API endpoints (Rocket)

Implemented in `src/api/semantic_search.rs` and mounted in `src/api/mod.rs`:

- `GET /api/projects/<id>/requirements/semantic_search?q=...&k=...`
  - Returns results, or `enabled=false` if embeddings are disabled.
- `POST /api/projects/<id>/requirements/ask`
  - Runs RAG if `RAG_ENABLED=true`, else `400`.
- `POST /api/projects/<id>/requirements/reindex` (**Admin only**)
  - Forces reindex for a project.
- `GET /api/projects/<id>/requirements/index_status`
  - Counts indexed vs pending vs failed.
- `GET /api/projects/<id>/requirements/semantic_search/status`
  - Returns current config bits (enabled flags + model names).

All endpoints are **project-scoped** and use the existing authorization guards (notably `ProjectAccess`; reindex also requires `AdminOnly`).

### 4.3 Automatic indexing (Rocket fairing)

`src/fairings/semantic_index.rs` attaches to Rocket liftoff (`src/app.rs` attaches `SemanticIndexFairing`).

Behavior:
- On startup: logs whether semantic search is enabled, then processes the queue once.
- In background: every 60 seconds, processes up to 50 pending queue items.

Queue items are created from:
- `src/services/requirement_service.rs` on create/update (best-effort: failures don’t block CRUD)
- `src/routes/html/excel.rs` after imports (best-effort)

---

## 5) Data model + database implementation

### 5.1 Postgres extensions and tables

Migration: `migrations/2026-01-29-000001_add_semantic_search/up.sql`

1) Enables `pgvector`:
- `CREATE EXTENSION IF NOT EXISTS vector;`

2) Adds **vector storage**:
- `requirement_embeddings` table:
  - `requirement_id` (PK, CASCADE delete)
  - `project_id` (scoping)
  - `embedding vector(...)` (dense embedding)
  - `embedding_model` (string)
  - `content_hash` (SHA-256 of embedding document + model)
  - `updated_at`

3) Adds **ANN index**:
- `USING hnsw (embedding vector_cosine_ops)`
  - Supports fast approximate nearest-neighbor lookups.

4) Adds **full-text search** support:
- `requirements.search_vector tsvector`
- Trigger `requirements_search_vector_trigger` updates `search_vector` on insert/update of:
  - `title`, `description`, `justification`, `reference_code`
- GIN index on `search_vector`

5) Adds **indexing queue**:
- `embedding_index_queue` table with statuses:
  - `pending`, `processing`, `completed`, `failed`
- Unique on `requirement_id` (only one job per requirement at a time)

### 5.2 Embedding dimensionality

The initial migration used `vector(1024)` to “support most models”.
Then `migrations/2026-01-30-144348-0000_fix_embedding_dimensions/up.sql` changes the column to:
- `ALTER TABLE requirement_embeddings ALTER COLUMN embedding TYPE vector(768);`

This matches the default embedding model `nomic-embed-text` (768 dimensions).

Implication:
- If you change `EMBEDDING_MODEL` to a 1024-dim model (e.g. `mxbai-embed-large`) you must also:
  - update `EMBEDDING_DIM`, **and**
  - migrate the DB back to `vector(1024)` (or to the correct dimension).

---

## 6) Retrieval implementation details

### 6.1 “Lexical” retrieval (FTS)

`SemanticSearchService::lexical_search()` (`src/services/semantic_search/search_service.rs`) runs SQL:
- `ts_rank_cd(search_vector, to_tsquery('english', $1)) AS rank`
- Filters by project_id and `search_vector @@ to_tsquery(...)`.

It builds a custom OR-based `tsquery` string by:
- Splitting words
- Dropping short words and a small stop-word list
- Adding `:*` prefix matching
- Joining with ` | ` (OR) for recall on “question-like” input

Notes:
- This improves recall for natural language queries, but `to_tsquery` is still strict syntax.

### 6.2 Vector retrieval (embeddings + cosine similarity)

`SemanticSearchService::vector_search()`:
1) Generates a query embedding via `EmbeddingProvider` (Ollama `/api/embed`).
2) Runs SQL:
   - `ORDER BY re.embedding <=> $1::vector` (cosine distance operator class)
   - Produces `similarity = (1 - distance)`

### 6.3 Fusion (RRF)

The two ranked lists are combined via Reciprocal Rank Fusion:
- Score contribution is `1 / (RRF_K + rank)` with `RRF_K = 60`
- This is “rank-only”: the underlying lexical rank score and vector similarity score are not directly mixed, which avoids scale-matching issues.

### 6.4 Exact reference code short-circuit

Before doing hybrid search, the service checks whether the query looks like a reference code (contains a dash + digits).
If it matches, it returns that requirement directly.

This improves UX for “REQ-001”-style navigation and avoids FTS parsing edge-cases for codes.

---

## 7) Indexing implementation details

### 7.1 Embedding document construction

`build_embedding_document()` (`src/services/semantic_search/document_builder.rs`) builds a deterministic text blob from `DecoratedRequirement`, including:
- Reference code
- Title
- Description
- Rationale/Justification
- Status/category/applicability/verification (human-readable strings)
- Parent title (if any)

This is hashed with the embedding model name to create `content_hash`.

Why it matters:
- Embeddings are only regenerated when content changed or the model changed.

### 7.2 Upsert logic

`IndexingService::index_requirement()`:
- Looks up existing embedding record (`requirement_embeddings`)
- Compares `content_hash` (or missing) to decide whether reindex is needed
- Writes via Diesel `insert_into(...).on_conflict(requirement_id).do_update().set(...)`

### 7.3 Background queue flow

When a requirement is created/updated/imported:
- A row in `embedding_index_queue` is upserted to `pending`.

The fairing:
- Picks oldest pending items first.
- Marks each as `processing`.
- Runs embedding generation + upsert.
- Marks as `completed` or `failed` with an error message.

---

## 8) RAG implementation details

### 8.1 What “Ask” does

`SemanticSearchService::ask()`:
1) Runs `search()` to retrieve top-k results (hybrid).
2) Builds prompts:
   - System prompt: “Use ONLY provided requirements… cite [REQ-XXX]…”
   - User prompt: includes requirement list with descriptions + metadata, then the question.
3) Calls `LlmProvider::chat()` to generate a response string.
4) Extracts citations by regex searching for `[REF-CODE]`.

### 8.2 Which model is used for RAG?

Defaults in `SemanticSearchConfig` (`src/services/semantic_search/config.rs`):
- `RAG_MODEL=llama3.2`
- `RAG_MAX_TOKENS=1024`

The LLM is served by Ollama at `OLLAMA_URL` (default `http://localhost:11434`).

### 8.3 Citation behavior is “best effort”

Citations are extracted only if the model emits bracketed reference codes matching actual results.
If the model omits them (or formats differently), `citations` may be empty even if the answer is grounded.

---

## 9) Frontend UX + integration points

### 9.1 Where the UI lives

The Requirements page includes:
- An “AI Search” button: `templates/requirements/_view_controls.html.hbs`
- A Bootstrap modal containing:
  - Query box
  - Optional filters (status/category/applicability/verification)
  - Result list and (optional) answer card
  - `Ctrl+K` shortcut hint
  - `templates/requirements/requirements.html.hbs`

The page also embeds:
- `#semanticSearchConfig` JSON with the current `projectId`.

### 9.2 Client behavior

`src/html/static/js/pages/semanticSearch.js`:
- On init, calls `/semantic_search/status` to detect enablement.
- On search:
  - Calls `/semantic_search?q=...&k=20&filters...`.
  - Renders results with rank and (if present) lexical/vector ranks.
- For “question-like” queries:
  - Calls `/ask` (best-effort; errors are swallowed/logged).

The “question-like” heuristic is:
- Starts with a question word (“what”, “how”, “does”, …) **or**
- Ends with `?`

---

## 10) Configuration and “which model are we using?”

Configuration is centralized in `SemanticSearchConfig::from_env()` (`src/services/semantic_search/config.rs`).

Key environment variables:

| Variable | Meaning | Default |
|---|---|---|
| `EMBEDDINGS_ENABLED` | Enable embedding generation + vector search | `false` |
| `EMBEDDING_PROVIDER` | `ollama` or `mock` | `ollama` |
| `EMBEDDING_MODEL` | Ollama embedding model | `nomic-embed-text` |
| `EMBEDDING_DIM` | Embedding dimension | auto (e.g. 768) |
| `OLLAMA_URL` | Ollama base URL | `http://localhost:11434` |
| `RAG_ENABLED` | Enable RAG answers | `false` |
| `RAG_MODEL` | Ollama chat model | `llama3.2` |
| `RAG_MAX_TOKENS` | Cap answer length | `1024` |
| `RAG_TOP_K` | Intended “context size” | `10` (currently unused by code) |

Important behavior:
- `SemanticSearchConfig::global()` uses a global `OnceLock`: env vars are read once at process start.

In this branch’s `.env`, embeddings + RAG are enabled by default (see `git diff origin/main..HEAD -- .env`), but code defaults are still “disabled unless env enables it”.

---

## 11) Known limitations / correctness risks

### 11.1 Model changes can silently degrade vector search until reindex

Vector search generates the **query embedding** using the current `EMBEDDING_MODEL`.
But vector retrieval does not currently filter stored embeddings by `embedding_model`.

If you change the embedding model (and don’t reindex yet), you can end up comparing embeddings from different vector spaces, producing poor results.

Mitigations:
- Force reindex whenever `EMBEDDING_MODEL` changes, or
- Filter `requirement_embeddings.embedding_model = current_model` in vector search (with an operational “reindex required” expectation).

### 11.2 LLM calls are blocking

`OllamaLlmProvider` uses `reqwest::blocking::Client` and `LlmProvider::chat()` is synchronous.
The async API handler (`ask`) awaits the async search, then calls the blocking LLM client, which can block request worker threads.

Mitigations:
- Use async reqwest and make LLM calls async, or
- Wrap blocking calls in `tokio::task::spawn_blocking`.

### 11.3 Unsafe lifetime cast in the fairing

`src/fairings/semantic_index.rs` uses `unsafe { std::mem::transmute(s) }` to treat Rocket-managed state as `'static` for background tasks.

This works if Rocket guarantees state lives for the duration of the process, but it is still an `unsafe` footgun:
- It makes it easier to introduce use-after-free if state ownership/lifetimes change.

Mitigation:
- Store an `Arc<AppState<...>>` as managed state and clone the Arc into background tasks (no unsafe cast).

### 11.4 `RAG_TOP_K` is defined but not used

The config supports `RAG_TOP_K`, but the code currently uses:
- UI: hardcoded `k=10` in `semanticSearch.js`
- API: defaults to 10 in `AskRequest`

---

## 12) Improvement opportunities (concrete)

### 12.1 Performance and scalability

1) **Reuse HTTP clients/providers**
   - `create_embedding_provider()` constructs a new `reqwest::Client` each time it’s called.
   - Indexing queue processing and vector search can call this frequently.
   - Suggested: keep providers in the service structs (or use an `Arc<Client>`/`OnceLock<Client>`), reuse per-request/per-service.

2) **Batch embedding generation in queue processing**
   - `EmbeddingProvider` already exposes `embed_batch()`, and Ollama’s `/api/embed` supports multiple inputs.
   - Current `process_queue()` embeds one requirement at a time.
   - Suggested: fetch N pending, build N documents, call `embed_batch()`, upsert all.

3) **Avoid formatting query embedding as a string**
   - `vector_search()` currently converts the embedding Vec<f32> into a `"[...]"` string and binds as `Text`.
   - Suggested: bind a `pgvector::Vector` parameter directly (if supported cleanly with Diesel + pgvector), or at least avoid repeated allocations.

4) **Push filters into SQL**
   - Both lexical and vector retrieval fetch result IDs, then filter in Rust when filters are set.
   - Suggested: include optional filters directly in SQL (dynamic SQL builder or Diesel boxed queries).

5) **Use `websearch_to_tsquery` for natural language**
   - The current `to_tsquery` approach is sensitive to token formatting.
   - `websearch_to_tsquery('english', query)` often behaves better for user-entered text.

### 12.2 Quality and correctness

1) **Enforce embedding model consistency**
   - Filter vector retrieval by `embedding_model`, and/or show “index needs rebuild” state when current model differs.

2) **Context length management for RAG**
   - Currently, the entire description is injected into the prompt.
   - For large projects and long descriptions, this can exceed context limits and degrade answer quality.
   - Suggested:
     - Truncate long fields,
     - Add per-requirement “snippet” selection for RAG,
     - Or chunk requirements and retrieve chunks.

3) **Citations enforcement**
   - Prompt asks for citations, but nothing enforces them.
   - Suggested:
     - Post-process: if answer has no citations, regenerate with a stricter prompt,
     - Or require the model to output JSON with structured citations, then validate.

4) **Use config knobs consistently**
   - Wire `RAG_TOP_K` into defaults (UI + API), and apply a global max.

### 12.3 Operational improvements

1) **Health checks**
   - There is an Ollama “tags” health check helper in `embedding_provider.rs`.
   - Suggested: expose an admin endpoint for “AI subsystem health” (DB + Ollama reachable + embedding dimension matches).

2) **Observability**
   - Add timing metrics to:
     - embedding generation latency
     - queue depth over time
     - search latency breakdown (lexical vs vector vs fusion)

3) **Backpressure + concurrency control**
   - If many requirements are updated/imported, the queue can grow.
   - Suggested: configurable batch size/interval; optional parallel embedding requests with a cap.

---

## 13) Summary

ReqMan’s semantic search is implemented as:
- **Hybrid retrieval** (Postgres FTS + pgvector cosine similarity) with **RRF** fusion,
- Optional **RAG answer generation** using **Ollama** (`nomic-embed-text` for embeddings, `llama3.2` for chat by default),
- A best-effort **indexing queue** processed by a Rocket fairing, triggered by requirement create/update/import.

The core architecture is solid and uses proven primitives (FTS + vector + RRF). The biggest high-value follow-ups are:
- remove blocking LLM calls from async paths,
- batch and reuse embedding requests,
- make embedding model/dimension consistency explicit and enforced,
- wire `RAG_TOP_K` and add context-length controls.

