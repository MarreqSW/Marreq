# Ollama Setup Guide for Semantic Search

This guide explains how to set up [Ollama](https://ollama.ai) for Marreq's semantic search feature. Ollama is an open-source tool that runs large language models locally.

## Why Ollama?

- **Privacy**: All data stays on your infrastructure
- **No API costs**: Run unlimited queries without usage fees
- **No vendor lock-in**: Use any compatible open-source model
- **Offline capable**: Works without internet after initial model download
- **Self-hosted**: Deploy on your own servers or locally

## Installation

### Linux

```bash
curl -fsSL https://ollama.ai/install.sh | sh
```

### macOS

```bash
brew install ollama
```

Or download from [ollama.ai/download](https://ollama.ai/download)

### Windows

Download the installer from [ollama.ai/download](https://ollama.ai/download)

### Docker

```bash
docker run -d -v ollama:/root/.ollama -p 11434:11434 --name ollama ollama/ollama
```

For GPU support (NVIDIA):
```bash
docker run -d --gpus=all -v ollama:/root/.ollama -p 11434:11434 --name ollama ollama/ollama
```

## Download Required Models

After installing Ollama, download the models needed for semantic search:

### Embedding Model (Required)

```bash
# Recommended: nomic-embed-text (768 dimensions, good quality/speed balance)
ollama pull nomic-embed-text

# Alternative: mxbai-embed-large (1024 dimensions, higher quality)
ollama pull mxbai-embed-large

# Alternative: all-minilm (384 dimensions, fastest)
ollama pull all-minilm
```

### LLM Model for RAG (Optional, for AI answers)

```bash
# Recommended: llama3.2 (good balance of quality and speed)
ollama pull llama3.2

# Alternative: mistral (fast, good for RAG)
ollama pull mistral

# Alternative: llama3.1 (higher quality, slower)
ollama pull llama3.1
```

## Verify Installation

Check that Ollama is running:

```bash
# Check server status
curl http://localhost:11434/api/tags

# Test embedding generation
curl http://localhost:11434/api/embed -d '{
  "model": "nomic-embed-text",
  "input": ["Hello world"]
}'

# Test chat (if using RAG)
curl http://localhost:11434/api/chat -d '{
  "model": "llama3.2",
  "messages": [{"role": "user", "content": "Hello"}],
  "stream": false
}'
```

## Configuration

Set these environment variables in your Marreq deployment:

### Basic Configuration (Embeddings Only)

```bash
# Enable semantic search
EMBEDDINGS_ENABLED=true

# Use Ollama provider
EMBEDDING_PROVIDER=ollama

# Embedding model (must match pulled model)
EMBEDDING_MODEL=nomic-embed-text

# Ollama server URL (default: http://localhost:11434)
OLLAMA_URL=http://localhost:11434
```

### Full Configuration (with RAG Answers)

```bash
# Embeddings
EMBEDDINGS_ENABLED=true
EMBEDDING_PROVIDER=ollama
EMBEDDING_MODEL=nomic-embed-text
OLLAMA_URL=http://localhost:11434

# RAG answer generation
RAG_ENABLED=true
RAG_MODEL=llama3.2
RAG_MAX_TOKENS=1024
RAG_TOP_K=10
```

### Example .env File

```bash
# Semantic Search Configuration
EMBEDDINGS_ENABLED=true
EMBEDDING_PROVIDER=ollama
EMBEDDING_MODEL=nomic-embed-text
OLLAMA_URL=http://localhost:11434

# RAG Configuration (optional)
RAG_ENABLED=true
RAG_MODEL=llama3.2
RAG_MAX_TOKENS=1024
```

## Model Recommendations

### Embedding Models

| Model | Dimensions | Size | Use Case |
|-------|------------|------|----------|
| `nomic-embed-text` | 768 | ~275MB | **Recommended** - Best quality/speed balance |
| `mxbai-embed-large` | 1024 | ~670MB | Higher quality, slower |
| `all-minilm` | 384 | ~45MB | Fastest, lower quality |
| `snowflake-arctic-embed` | 1024 | ~670MB | Good for technical content |

### LLM Models (for RAG)

| Model | Size | Use Case |
|-------|------|----------|
| `llama3.2` | ~2GB | **Recommended** - Fast, good quality |
| `llama3.2:1b` | ~1.3GB | Faster, lower quality |
| `mistral` | ~4GB | Good for structured responses |
| `llama3.1` | ~4.7GB | Higher quality, slower |
| `llama3.1:70b` | ~40GB | Best quality, requires powerful GPU |

## Hardware Requirements

### Minimum (CPU only)
- 8GB RAM
- 10GB disk space
- Any modern CPU

### Recommended (with GPU)
- 16GB RAM
- NVIDIA GPU with 8GB+ VRAM
- 20GB disk space

### For Large Models (llama3.1:70b)
- 64GB+ RAM or
- NVIDIA GPU with 48GB+ VRAM

## Production Deployment

### Docker Compose

Add Ollama to your `docker-compose.yml`:

```yaml
services:
  ollama:
    image: ollama/ollama
    ports:
      - "11434:11434"
    volumes:
      - ollama_data:/root/.ollama
    # For GPU support, uncomment:
    # deploy:
    #   resources:
    #     reservations:
    #       devices:
    #         - capabilities: [gpu]
    restart: unless-stopped

  Marreq:
    # ... your Marreq configuration
    environment:
      - EMBEDDINGS_ENABLED=true
      - EMBEDDING_PROVIDER=ollama
      - EMBEDDING_MODEL=nomic-embed-text
      - OLLAMA_URL=http://ollama:11434
      - RAG_ENABLED=true
      - RAG_MODEL=llama3.2
    depends_on:
      - ollama

volumes:
  ollama_data:
```

### Initialize Models on Startup

Create a startup script to ensure models are downloaded:

```bash
#!/bin/bash
# init-ollama.sh

# Wait for Ollama to be ready
until curl -s http://localhost:11434/api/tags > /dev/null; do
  echo "Waiting for Ollama..."
  sleep 2
done

# Pull required models
ollama pull nomic-embed-text
ollama pull llama3.2

echo "Ollama models ready"
```

### Kubernetes

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: ollama
spec:
  replicas: 1
  selector:
    matchLabels:
      app: ollama
  template:
    metadata:
      labels:
        app: ollama
    spec:
      containers:
      - name: ollama
        image: ollama/ollama
        ports:
        - containerPort: 11434
        volumeMounts:
        - name: ollama-data
          mountPath: /root/.ollama
        resources:
          requests:
            memory: "8Gi"
            cpu: "2"
          limits:
            memory: "16Gi"
            cpu: "4"
            # nvidia.com/gpu: 1  # Uncomment for GPU
      volumes:
      - name: ollama-data
        persistentVolumeClaim:
          claimName: ollama-pvc
---
apiVersion: v1
kind: Service
metadata:
  name: ollama
spec:
  selector:
    app: ollama
  ports:
  - port: 11434
    targetPort: 11434
```

## Troubleshooting

### "Ollama server not reachable"

1. Check if Ollama is running:
   ```bash
   systemctl status ollama  # Linux with systemd
   # or
   ollama serve  # Manual start
   ```

2. Verify the URL is correct:
   ```bash
   curl http://localhost:11434/api/tags
   ```

3. Check firewall/network settings if using remote Ollama

### "Model not found"

Pull the required model:
```bash
ollama pull nomic-embed-text
ollama pull llama3.2
```

List available models:
```bash
ollama list
```

### Slow Performance

1. **Use GPU**: Ollama is much faster with GPU acceleration
2. **Use smaller models**: Try `all-minilm` for embeddings, `llama3.2:1b` for LLM
3. **Increase resources**: Give Ollama more CPU/RAM
4. **Reduce batch size**: Process fewer requirements at once

### Out of Memory

1. Use smaller models
2. Increase system swap space
3. Use GPU with sufficient VRAM
4. Reduce `RAG_TOP_K` to send less context to LLM

### Docker: GPU Not Detected

Ensure NVIDIA Container Toolkit is installed:
```bash
# Ubuntu/Debian
distribution=$(. /etc/os-release;echo $ID$VERSION_ID)
curl -s -L https://nvidia.github.io/nvidia-docker/gpgkey | sudo apt-key add -
curl -s -L https://nvidia.github.io/nvidia-docker/$distribution/nvidia-docker.list | \
  sudo tee /etc/apt/sources.list.d/nvidia-docker.list
sudo apt-get update
sudo apt-get install -y nvidia-container-toolkit
sudo systemctl restart docker
```

## Security Considerations

1. **Network isolation**: Don't expose Ollama to the public internet
2. **Resource limits**: Set CPU/memory limits to prevent DoS
3. **Model verification**: Only use models from trusted sources
4. **Data handling**: Requirement text is sent to Ollama for processing

## Further Reading

- [Ollama Documentation](https://github.com/ollama/ollama/blob/main/docs/README.md)
- [Ollama Model Library](https://ollama.ai/library)
- [Ollama API Reference](https://github.com/ollama/ollama/blob/main/docs/api.md)
