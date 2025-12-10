---
name: setup-environment
description: Set up ONNX runtime and embedding model weights for semantic search. Use when the build fails with ONNX errors, model download issues, or environment variable problems.
---

# Environment Setup for Semantic Search

## Instructions

1. Download ONNX Runtime from GitHub releases
2. Download embedding model from HuggingFace
3. Set required environment variables
4. Verify setup with cargo test

## Quick Setup

Run the automated script:
```bash
./scripts/setup-models.sh
```

## Manual Setup

### 1. Download ONNX Runtime

```bash
ONNX_VERSION="1.20.0"
PLATFORM="linux-x64"  # or: linux-aarch64, osx-x86_64, osx-arm64

curl -L -o /tmp/onnxruntime.tgz \
  "https://github.com/microsoft/onnxruntime/releases/download/v${ONNX_VERSION}/onnxruntime-${PLATFORM}-${ONNX_VERSION}.tgz"

mkdir -p ~/.cache/glhf
tar -xzf /tmp/onnxruntime.tgz -C ~/.cache/glhf/
```

### 2. Download Embedding Model

```bash
MODEL_DIR=~/.cache/huggingface/hub/models--Qdrant--all-MiniLM-L6-v2-onnx
COMMIT=5f1b8cd78bc4fb444dd171e59b18f3a3af89a079

mkdir -p "$MODEL_DIR/snapshots/$COMMIT" "$MODEL_DIR/refs"

HF_BASE="https://huggingface.co/Qdrant/all-MiniLM-L6-v2-onnx/resolve/main"
curl -L -o "$MODEL_DIR/snapshots/$COMMIT/model.onnx" "$HF_BASE/model.onnx"
curl -L -o "$MODEL_DIR/snapshots/$COMMIT/tokenizer.json" "$HF_BASE/tokenizer.json"
curl -L -o "$MODEL_DIR/snapshots/$COMMIT/config.json" "$HF_BASE/config.json"
curl -L -o "$MODEL_DIR/snapshots/$COMMIT/special_tokens_map.json" "$HF_BASE/special_tokens_map.json"
curl -L -o "$MODEL_DIR/snapshots/$COMMIT/tokenizer_config.json" "$HF_BASE/tokenizer_config.json"

# IMPORTANT: No trailing newline in refs/main
printf '%s' "$COMMIT" > "$MODEL_DIR/refs/main"
```

### 3. Set Environment Variables

```bash
export ORT_LIB_LOCATION="$HOME/.cache/glhf/onnxruntime-linux-x64-1.20.0/lib"
export ORT_STRATEGY=system
export LD_LIBRARY_PATH="$ORT_LIB_LOCATION:$LD_LIBRARY_PATH"
export HF_HOME="$HOME/.cache/huggingface/hub"
```

### 4. Verify

```bash
cargo test embed -- --ignored
```

## Common Issues

| Error | Solution |
|-------|----------|
| `failed to load onnxruntime` | Check ORT_LIB_LOCATION points to lib/ directory |
| `model.onnx not found` | Verify refs/main has no trailing newline |
| `TLS certificate error` | Use `curl --insecure` in sandboxed environments |
