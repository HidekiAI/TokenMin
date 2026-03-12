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

if ! command -v wasm-pack >/dev/null 2>&1; then
  echo "[setup_rust.sh] Installing wasm-pack for WebAssembly support..."
  WASM_PACK_INSTALLER="/tmp/wasm-pack-init.sh"
  echo "[SECURITY] This will download and execute the official wasm-pack installer script from:"
  echo "[SECURITY]   https://rustwasm.github.io/wasm-pack/installer/init.sh"
  echo "[SECURITY] You can review the installer before proceeding, for example:"
  echo "[SECURITY]   curl -sSf https://rustwasm.github.io/wasm-pack/installer/init.sh -o wasm-pack-init.sh"
  echo "[SECURITY]   less wasm-pack-init.sh"
  read -r -p "[setup_rust.sh] Proceed with downloading and running the wasm-pack installer? [y/N]: " WASM_PACK_CONFIRM
  case "${WASM_PACK_CONFIRM:-}" in
    [yY])
      curl --proto '=https' --tlsv1.2 -sSf https://rustwasm.github.io/wasm-pack/installer/init.sh -o "$WASM_PACK_INSTALLER"
      sh "$WASM_PACK_INSTALLER"
      ;;
    *)
      echo "[setup_rust.sh] Skipping automatic wasm-pack installation at user request."
      echo "[setup_rust.sh] To install manually, review and run the official installer script from:"
      echo "[setup_rust.sh]   https://rustwasm.github.io/wasm-pack/installer/init.sh"
      ;;
  esac
else
  echo "[setup_rust.sh] wasm-pack is already installed."
fi

LATEST=$(rustc --version)
echo "[setup_rust.sh] Rust toolchain installed and set to stable: $LATEST."
