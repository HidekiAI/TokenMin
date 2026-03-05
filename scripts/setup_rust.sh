#!/usr/bin/env bash
# setup_rust.sh - Install or update Rust toolchain for TokenMin (Linux only)
set -euo pipefail
if ! command -v rustc >/dev/null 2>&1; then
  echo "Rust not found. Installing Rustup and Rust toolchain..."
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  source "$HOME/.cargo/env"
else
  echo "Rust is already installed. Checking for updates..."
  rustup self update || true
fi
rustup install stable
rustup update stable
rustup default stable
LATEST=$(rustc --version)
echo "Rust toolchain installed and set to stable: $LATEST."
