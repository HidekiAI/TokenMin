#!/usr/bin/env bash
# setup_ollama.sh - Install and configure Ollama for TokenMin
set -euo pipefail

MODEL_NAME="${OLLAMA_MODEL:-qwen2.5-coder:0.5b}"

install_ollama() {
  if command -v ollama >/dev/null 2>&1; then
    echo "[setup_ollama.sh] Ollama is already installed. Checking for updates..."
    echo "[SECURITY] This script uses the official 'curl | sh' installer from ollama.com."
    curl -fsSL https://ollama.com/install.sh | sh
  else
    echo "[setup_ollama.sh] Ollama not found. Installing..."
    echo "[SECURITY] This script uses the official 'curl | sh' installer from ollama.com."
    curl -fsSL https://ollama.com/install.sh | sh
  fi
}

# The official installer handles everything on Linux.
# For other OSs or custom paths, user should install manually or we can expand this.
install_ollama

echo "[setup_ollama.sh] Ensuring Ollama is running..."
# Check if ollama is already running
if ! pgrep -x "ollama" > /dev/null; then
  echo "[setup_ollama.sh] Starting Ollama server in background..."
  ollama serve > /tmp/ollama.log 2>&1 &
  sleep 5
else
  echo "[setup_ollama.sh] Ollama server is already running."
fi

echo "[setup_ollama.sh] Pulling minimal model: $MODEL_NAME ..."
ollama pull "$MODEL_NAME"

echo "[setup_ollama.sh] Ollama setup complete with model $MODEL_NAME."
