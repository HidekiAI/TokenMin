#!/usr/bin/env bash
# setup_rust.sh - Install or update Rust toolchain for TokenMin (Linux only)
set -euo pipefail

echo "[setup_rust.sh] Validating Rust toolchain..."

if ! command -v rustc >/dev/null 2>&1; then
  echo "[setup_rust.sh] Rust not found. Installing Rustup and Rust toolchain..."
  echo "[SECURITY] This script uses the official 'curl | sh' installer from sh.rustup.rs."
  echo "[SECURITY] You can verify the installer script before proceeding: curl -sSf https://sh.rustup.rs > rustup-init.sh"
  
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
  source "$HOME/.cargo/env"
else
  echo "[setup_rust.sh] Rust is already installed. Checking for updates..."
  rustup self update || true
fi

rustup install stable
rustup update stable
rustup default stable

LATEST=$(rustc --version)
echo "[setup_rust.sh] Rust toolchain installed and set to stable: $LATEST."
