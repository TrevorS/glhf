#!/bin/bash
# Setup script for ONNX runtime and fastembed model weights
# Run this script to download the required dependencies for semantic search

set -e

# Configuration
ONNX_VERSION="1.20.0"
MODEL_REPO="Qdrant/all-MiniLM-L6-v2-onnx"
MODEL_COMMIT="5f1b8cd78bc4fb444dd171e59b18f3a3af89a079"

# Detect platform
PLATFORM=$(uname -s)
ARCH=$(uname -m)

case "$PLATFORM-$ARCH" in
    Linux-x86_64)
        ONNX_PLATFORM="linux-x64"
        ;;
    Linux-aarch64)
        ONNX_PLATFORM="linux-aarch64"
        ;;
    Darwin-x86_64)
        ONNX_PLATFORM="osx-x86_64"
        ;;
    Darwin-arm64)
        ONNX_PLATFORM="osx-arm64"
        ;;
    *)
        echo "Unsupported platform: $PLATFORM-$ARCH"
        exit 1
        ;;
esac

# Directories
CACHE_DIR="${HOME}/.cache/glhf"
ONNX_DIR="${CACHE_DIR}/onnxruntime-${ONNX_PLATFORM}-${ONNX_VERSION}"
HF_CACHE="${HOME}/.cache/huggingface/hub"
MODEL_DIR="${HF_CACHE}/models--Qdrant--all-MiniLM-L6-v2-onnx"

echo "=== glhf Model Setup ==="
echo "Platform: $PLATFORM-$ARCH"
echo ""

# Download ONNX Runtime
if [ -d "$ONNX_DIR" ] && [ -f "$ONNX_DIR/lib/libonnxruntime.so" -o -f "$ONNX_DIR/lib/libonnxruntime.dylib" ]; then
    echo "✓ ONNX Runtime ${ONNX_VERSION} already installed"
else
    echo "Downloading ONNX Runtime ${ONNX_VERSION}..."
    mkdir -p "$CACHE_DIR"

    ONNX_URL="https://github.com/microsoft/onnxruntime/releases/download/v${ONNX_VERSION}/onnxruntime-${ONNX_PLATFORM}-${ONNX_VERSION}.tgz"
    TMP_FILE=$(mktemp)

    curl -L -o "$TMP_FILE" "$ONNX_URL"
    tar -xzf "$TMP_FILE" -C "$CACHE_DIR"
    rm "$TMP_FILE"

    echo "✓ ONNX Runtime installed to $ONNX_DIR"
fi

# Download fastembed model
SNAPSHOT_DIR="${MODEL_DIR}/snapshots/${MODEL_COMMIT}"
REFS_DIR="${MODEL_DIR}/refs"

if [ -f "${SNAPSHOT_DIR}/model.onnx" ] && [ -f "${SNAPSHOT_DIR}/tokenizer.json" ]; then
    echo "✓ Embedding model already downloaded"
else
    echo "Downloading embedding model (all-MiniLM-L6-v2, ~90MB)..."
    mkdir -p "$SNAPSHOT_DIR"
    mkdir -p "$REFS_DIR"

    HF_BASE="https://huggingface.co/${MODEL_REPO}/resolve/main"

    # Download model files
    curl -L -o "${SNAPSHOT_DIR}/model.onnx" "${HF_BASE}/model.onnx"
    curl -L -o "${SNAPSHOT_DIR}/tokenizer.json" "${HF_BASE}/tokenizer.json"
    curl -L -o "${SNAPSHOT_DIR}/config.json" "${HF_BASE}/config.json"
    curl -L -o "${SNAPSHOT_DIR}/special_tokens_map.json" "${HF_BASE}/special_tokens_map.json"
    curl -L -o "${SNAPSHOT_DIR}/tokenizer_config.json" "${HF_BASE}/tokenizer_config.json"

    # Write refs/main (no trailing newline)
    printf '%s' "$MODEL_COMMIT" > "${REFS_DIR}/main"

    echo "✓ Embedding model installed"
fi

echo ""
echo "=== Setup Complete ==="
echo ""
echo "To use glhf with semantic search, set these environment variables:"
echo ""
echo "  export ORT_LIB_LOCATION=\"${ONNX_DIR}/lib\""
echo "  export ORT_STRATEGY=system"
echo "  export LD_LIBRARY_PATH=\"${ONNX_DIR}/lib:\$LD_LIBRARY_PATH\""
echo "  export HF_HOME=\"${HF_CACHE}\""
echo ""
echo "Or add this to your shell profile (~/.bashrc, ~/.zshrc, etc.):"
echo ""
cat << EOF
# glhf semantic search configuration
export ORT_LIB_LOCATION="${ONNX_DIR}/lib"
export ORT_STRATEGY=system
export LD_LIBRARY_PATH="${ONNX_DIR}/lib:\$LD_LIBRARY_PATH"
export HF_HOME="${HF_CACHE}"
EOF
