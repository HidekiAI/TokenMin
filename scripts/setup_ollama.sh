#!/usr/bin/env bash
# setup_ollama.sh - Install and configure Ollama for TokenMin
set -euo pipefail

MODEL_NAME="${OLLAMA_MODEL:-qwen2.5-coder:0.5b}"

install_ollama() {
  if command -v ollama >/dev/null 2>&1; then
    echo "Ollama is already installed. Checking for updates..."
    curl -fsSL https://ollama.com/install.sh | sh
  else
    echo "Ollama not found. Installing..."
    curl -fsSL https://ollama.com/install.sh | sh
  fi
}

# The official installer handles everything on Linux.
# For other OSs or custom paths, user should install manually or we can expand this.
install_ollama

echo "Ensuring Ollama is running..."
# Check if ollama is already running
if ! pgrep -x "ollama" > /dev/null; then
  echo "Starting Ollama server in background..."
  ollama serve > /tmp/ollama.log 2>&1 &
  sleep 5
else
  echo "Ollama server is already running."
fi

echo "Pulling minimal model: $MODEL_NAME ..."
ollama pull "$MODEL_NAME"

echo "Ollama setup complete with model $MODEL_NAME."
