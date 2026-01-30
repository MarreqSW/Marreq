# Semantic Search (AI) for Requirements

ReqMan includes an optional RAG-powered semantic search feature that enables:
- **Semantic/hybrid search**: Find requirements by meaning, not just keywords
- **RAG answers**: Ask questions and get AI-generated answers grounded in your requirements with citations

All AI processing runs locally using [Ollama](https://ollama.ai), ensuring your data stays private and secure.

## Quick Start

1. **Install Ollama** (see [OLLAMA_SETUP.md](OLLAMA_SETUP.md) for details):
   ```bash
   curl -fsSL https://ollama.ai/install.sh | sh
   ```

2. **Download required models**:
   ```bash
   ollama pull nomic-embed-text  # For embeddings
   ollama pull llama3.2          # For RAG answers (optional)
   ```

3. **Run database migration**:
   ```bash
   diesel migration run
   ```

4. **Configure environment**:
   ```bash
   EMBEDDINGS_ENABLED=true
   EMBEDDING_PROVIDER=ollama
   EMBEDDING_MODEL=nomic-embed-text
   OLLAMA_URL=http://localhost:11434
   RAG_ENABLED=true
   RAG_MODEL=llama3.2
   ```

5. **Reindex requirements** (admin only):
   ```bash
   curl -X POST http://localhost:8000/api/projects/1/requirements/reindex
   ```

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
