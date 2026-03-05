#!/usr/bin/env bash
# setup_ollama.sh - Install and configure Ollama for TokenMin (Linux only)
set -euo pipefail
MODEL_NAME="qwen:2.5b"
REQUIRED_VERSION="0.1.34" # Example version, update as needed

choose_install_dir() {
  if command -v ollama >/dev/null 2>&1; then
    OLLAMA_PATH="$(which ollama)"
    echo "Ollama is already installed at $OLLAMA_PATH and is in your PATH. Skipping installation, will only pull model."
    INSTALL_DIR="$(dirname "$OLLAMA_PATH")"
    SKIP_OLLAMA_INSTALL=1
    exit 0
  fi
  if [[ -d "$HOME/local/bin" ]]; then
    if [[ ":$PATH:" != *":$HOME/local/bin:"* ]]; then
      echo "~/local/bin exists but is not in your PATH. This is required so TokenMin can find and run Ollama. Please add it to your PATH before running setup again."
      exit 1
    fi
    echo "Using existing ~/local/bin for Ollama install."
    INSTALL_DIR="$HOME/local/bin"
    exit 0
  fi
  read -p "~/local/bin does not exist. Create it? [y/N]: " create_local
  if [[ "$create_local" =~ ^[Yy]$ ]]; then
    mkdir -p "$HOME/local/bin"
    if [[ ":$PATH:" != *":$HOME/local/bin:"* ]]; then
      echo "~/local/bin created but is not in your PATH. This is required so TokenMin can find and run Ollama. Please add it to your PATH before running setup again."
      exit 1
    fi
    echo "Created ~/local/bin."
    INSTALL_DIR="$HOME/local/bin"
    exit 0
  fi
  if [[ -d "/usr/local/bin" ]]; then
    if [[ ":$PATH:" != *":/usr/local/bin:"* ]]; then
      echo "/usr/local/bin exists but is not in your PATH. This is required so TokenMin can find and run Ollama. Please add it to your PATH before running setup again."
      exit 1
    fi
    echo "Using existing /usr/local/bin for Ollama install."
    INSTALL_DIR="/usr/local/bin"
    exit 0
  fi
  read -p "/usr/local/bin does not exist. Create it? [y/N]: " create_usr
  if [[ "$create_usr" =~ ^[Yy]$ ]]; then
    sudo mkdir -p "/usr/local/bin"
    if [[ ":$PATH:" != *":/usr/local/bin:"* ]]; then
      echo "/usr/local/bin created but is not in your PATH. This is required so TokenMin can find and run Ollama. Please add it to your PATH before running setup again."
      exit 1
    fi
    echo "Created /usr/local/bin."
    INSTALL_DIR="/usr/local/bin"
    exit 0
  fi
  read -p "Specify your preferred install directory: " custom_dir
  if [[ ! -d "$custom_dir" ]]; then
    echo "Directory $custom_dir does not exist. Please create it and ensure it is in your PATH before running setup again."
    exit 1
  fi
  if [[ ":$PATH:" != *":$custom_dir:"* ]]; then
    echo "Directory $custom_dir is not in your PATH. This is required so TokenMin can find and run Ollama. Please add it to your PATH before running setup again."
    exit 1
  fi
  echo "Using $custom_dir for Ollama install."
  INSTALL_DIR="$custom_dir"
  exit 0
}

choose_install_dir


INSTALL_DIR="$(choose_install_dir)"

if command -v ollama >/dev/null 2>&1; then
  INSTALLED_VERSION="$(ollama --version | awk '{print $2}')"
  echo "Ollama found: version $INSTALLED_VERSION"
  if [[ "$INSTALLED_VERSION" != "$REQUIRED_VERSION" ]]; then
    echo "Ollama version mismatch (required: $REQUIRED_VERSION). Reinstalling..."
    curl -fsSL https://ollama.com/install.sh | sh
  else
    echo "Ollama version matches required version."
  fi
else
  echo "Ollama not found. Installing..."
  curl -fsSL https://ollama.com/install.sh | sh
fi

echo "Ensuring Ollama is running..."
ollama serve &
sleep 5

echo "Pulling minimal model: $MODEL_NAME ..."
ollama pull "$MODEL_NAME"

echo "Ollama installed and model $MODEL_NAME pulled. Run 'ollama list' to verify."
