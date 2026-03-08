#!/usr/bin/env bash
# setup_ollama.sh - Install and configure Ollama for TokenMin
set -euo pipefail

MODEL_NAME="${OLLAMA_MODEL:-qwen2.5-coder:0.5b}"

OS_TYPE="$(uname -s 2>/dev/null || echo unknown)"
case "$OS_TYPE" in
  Linux*)
    echo "[setup_ollama.sh] Detected Linux ($OS_TYPE). Proceeding with Ollama installation."
    ;;
  *)
    echo "[setup_ollama.sh] Unsupported operating system: $OS_TYPE"
    echo "[setup_ollama.sh] This setup script currently supports only Linux using the official ollama.com installer."
    echo "[setup_ollama.sh] Please install Ollama manually for your platform and configure it before rerunning this setup."
    exit 1
    ;;
esac

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

