# Semantic Search (AI) for Requirements

ReqMan includes an optional RAG-powered semantic search feature that enables:
- **Semantic/hybrid search**: Find requirements by meaning, not just keywords
- **RAG answers**: Ask questions and get AI-generated answers grounded in your requirements with citations
- **Automatic indexing**: Requirements are automatically queued for embedding generation on create/update

All AI processing runs locally using [Ollama](https://ollama.ai), ensuring your data stays private and secure.

## Quick Start

### 1. Start PostgreSQL with pgvector (Docker)

Use the provided `docker-compose.yml` which includes pgvector:

```bash
docker-compose up -d db
```

This starts PostgreSQL 15 with the pgvector extension pre-installed.

### 2. Install Ollama

See [OLLAMA_SETUP.md](OLLAMA_SETUP.md) for details:

```bash
# Linux/WSL
curl -fsSL https://ollama.ai/install.sh | sh

# macOS
brew install ollama

# Start the server
ollama serve
```

### 3. Download required models

```bash
ollama pull nomic-embed-text  # For embeddings (768 dimensions)
ollama pull llama3.2          # For RAG answers (optional)
```

### 4. Run database migrations

```bash
diesel migration run
```

### 5. Configure environment

Add to your `.env` file:

```bash
# Required for semantic search
EMBEDDINGS_ENABLED=true
EMBEDDING_PROVIDER=ollama
OLLAMA_URL=http://localhost:11434

# Optional: RAG answers
RAG_ENABLED=true
RAG_MODEL=llama3.2
```

### 6. Start ReqMan

```bash
cargo run
```

On startup, you'll see:
```
🔍 Semantic search enabled: provider=ollama, model=nomic-embed-text, dim=768
🤖 RAG enabled: model=llama3.2, max_tokens=1024
✅ Semantic index background processor started
```

### 7. Initial reindex (for existing requirements)

For projects with existing requirements, trigger a one-time reindex:

```bash
# Via the UI (Admin menu → Reindex)
# Or via API:
curl -X POST http://localhost:8000/api/projects/1/requirements/reindex \
  -H "Cookie: session=<your-session-cookie>"
```

After initial reindex, new/updated requirements are automatically indexed.

## Architecture

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
│                    Search Service                               │
│  ┌──────────────┐   ┌──────────────┐   ┌──────────────┐        │
│  │   Lexical    │   │    Vector    │   │     RRF      │        │
│  │   Search     │ + │   Search     │ → │   Fusion     │        │
│  │  (tsvector)  │   │  (pgvector)  │   │              │        │
│  └──────────────┘   └──────────────┘   └──────────────┘        │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                    Ollama (Local AI)                            │
│  ┌──────────────────┐   ┌──────────────────┐                   │
│  │  nomic-embed-text │   │    llama3.2      │                   │
│  │   (embeddings)    │   │   (RAG answers)  │                   │
│  └──────────────────┘   └──────────────────┘                   │
└─────────────────────────────────────────────────────────────────┘
                              │
                              ▼
┌─────────────────────────────────────────────────────────────────┐
│                      PostgreSQL                                 │
│  requirements (search_vector tsvector)                          │
│  requirement_embeddings (embedding vector)                      │
└─────────────────────────────────────────────────────────────────┘
```

## Requirements

### PostgreSQL with pgvector

Install the [pgvector](https://github.com/pgvector/pgvector) extension:

```bash
# Ubuntu/Debian
sudo apt install postgresql-16-pgvector

# macOS
brew install pgvector

# Docker
docker pull pgvector/pgvector:pg16
```

### Ollama

See [OLLAMA_SETUP.md](OLLAMA_SETUP.md) for complete installation instructions.

## Configuration

### Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `EMBEDDINGS_ENABLED` | Enable embedding generation | `false` |
| `EMBEDDING_PROVIDER` | Provider (`ollama` or `mock`) | `ollama` |
| `EMBEDDING_MODEL` | Ollama embedding model | `nomic-embed-text` |
| `EMBEDDING_DIM` | Embedding dimension (auto-detected) | `768` |
| `OLLAMA_URL` | Ollama server URL | `http://localhost:11434` |
| `RAG_ENABLED` | Enable RAG answer generation | `false` |
| `RAG_MODEL` | Ollama LLM model | `llama3.2` |
| `RAG_MAX_TOKENS` | Max tokens for RAG response | `1024` |
| `RAG_TOP_K` | Results to use for RAG context | `10` |

### Recommended Ollama Models

**Embedding Models:**
| Model | Dimensions | Best For |
|-------|------------|----------|
| `nomic-embed-text` | 768 | General use (recommended) |
| `mxbai-embed-large` | 1024 | Higher quality |
| `all-minilm` | 384 | Fastest |

**LLM Models (for RAG):**
| Model | Best For |
|-------|----------|
| `llama3.2` | General use (recommended) |
| `mistral` | Fast responses |
| `llama3.1` | Higher quality |

## Usage

### User Interface

1. Navigate to the Requirements page for any project
2. Click the **AI Search** button (or press `Ctrl+K` / `Cmd+K`)
3. Enter your search query:
   - **Keywords**: Find requirements matching terms
   - **Questions**: Get AI-generated answers (e.g., "What are the safety requirements?")
   - **Reference codes**: Direct lookup (e.g., "REQ-001")
4. Optionally expand **Filters** to narrow results
5. Results show:
   - **Ranked requirements** with match scores
   - **AI Answer** (for question queries) with citations

### API Endpoints

#### Search Requirements

```http
GET /api/projects/{project_id}/requirements/semantic_search?q=<query>&k=<limit>
```

Response:
```json
{
  "enabled": true,
  "results": [
    {
      "id": 1,
      "reference_code": "REQ-001",
      "title": "System shall...",
      "snippet": "...",
      "score": 0.95,
      "rank": 1,
      "status": "Draft",
      "category": "Functional"
    }
  ],
  "total": 1
}
```

#### Ask a Question (RAG)

```http
POST /api/projects/{project_id}/requirements/ask
Content-Type: application/json

{
  "query": "What are the safety requirements?",
  "k": 10
}
```

Response:
```json
{
  "answer": "Based on the requirements, safety is addressed by [REQ-SAF-001] which specifies...",
  "citations": [
    {"requirement_id": 5, "reference_code": "REQ-SAF-001", "title": "Safety Constraint"}
  ],
  "results": [...]
}
```

#### Reindex Project (Admin Only)

```http
POST /api/projects/{project_id}/requirements/reindex
```

#### Get Index Status

```http
GET /api/projects/{project_id}/requirements/index_status
```

## How It Works

### Hybrid Search

The search combines two retrieval methods using Reciprocal Rank Fusion (RRF):

1. **Lexical Search** (PostgreSQL full-text search)
   - Uses `tsvector` for fast keyword matching
   - Good for exact matches and known terminology

2. **Vector Search** (pgvector similarity)
   - Computes cosine similarity between query and requirement embeddings
   - Good for semantic meaning and paraphrased queries

3. **RRF Fusion**
   - Combines results: `score = sum(1 / (k + rank))`
   - Balances lexical precision with semantic recall

### Embedding Document

For each requirement, an embedding document is created:

```
[REF] REQ-SYS-001
[TITLE] System shall process inputs
[DESC] The system shall process all valid inputs within 100ms
[RATIONALE] Required for real-time operation
[CATEGORY] Functional
[STATUS] Draft
```

A SHA-256 content hash ensures embeddings are only regenerated when content changes.

## Automatic Indexing

ReqMan automatically manages embeddings for requirements:

### When Indexing Happens

| Event | Behavior |
|-------|----------|
| **Create requirement** | Queued for indexing immediately |
| **Update requirement** | Queued for re-indexing (only if content changed) |
| **Delete requirement** | Embedding deleted via CASCADE |
| **Import from Excel/CSV** | All imported requirements queued |
| **Application startup** | Pending queue items processed |

### Background Processing

A background task runs every 60 seconds to process queued items:
- Processes up to 50 items per run
- Skips requirements whose content hasn't changed (using content hash)
- Logs progress: `📊 Background index queue: processed=5, failed=0`

### Index Queue States

| Status | Description |
|--------|-------------|
| `pending` | Waiting to be processed |
| `processing` | Currently generating embedding |
| `completed` | Successfully indexed |
| `failed` | Error occurred (will be retried on next reindex) |

### Monitoring Index Status

```bash
# Check project indexing status
curl http://localhost:8000/api/projects/1/requirements/index_status

# Response:
{
  "project_id": 1,
  "total_requirements": 150,
  "indexed_count": 148,
  "pending_count": 2,
  "failed_count": 0,
  "embeddings_enabled": true,
  "embedding_model": "nomic-embed-text"
}
```

## Troubleshooting

### "Ollama server not reachable"
- Start Ollama: `ollama serve`
- Check URL: `curl http://localhost:11434/api/tags`

### "Model not found"
- Pull the model: `ollama pull nomic-embed-text`

### "pgvector extension not found"
- Install pgvector for your PostgreSQL version

### Slow Performance
- Use GPU acceleration for Ollama
- Use smaller models (`all-minilm`, `llama3.2:1b`)

## Security

1. **Data Privacy**: All processing is local - no data leaves your infrastructure
2. **Project Isolation**: Queries are scoped to user's accessible projects
3. **Network Security**: Don't expose Ollama to public internet

## Development

### Running Tests

```bash
# Rust tests
cargo test semantic_search --lib

# With mock provider (no Ollama needed)
EMBEDDING_PROVIDER=mock cargo test
```

### Mock Provider

For development without Ollama:
```bash
EMBEDDINGS_ENABLED=true
EMBEDDING_PROVIDER=mock
```

The mock provider generates deterministic embeddings for testing.
