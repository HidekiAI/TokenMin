#!/usr/bin/env bash
# setup.sh - Wrapper to set up all dependencies for TokenMin (Linux/macOS/Windows with compatible tools)
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
MODE="local"
if [[ $# -gt 0 ]]; then
  MODE="$1"
fi

OS_TYPE="$(uname -s)"
case "$OS_TYPE" in
  Linux)
    COMPATIBLE=1
    ;;
  Darwin)
    # macOS: require bash, curl, unzip, docker
    if command -v bash >/dev/null 2>&1 && command -v curl >/dev/null 2>&1 && command -v unzip >/dev/null 2>&1 && command -v docker >/dev/null 2>&1; then
      COMPATIBLE=1
    else
      echo "[setup.sh] macOS detected, but required tools (bash, curl, unzip, docker) are missing."
      COMPATIBLE=0
    fi
    ;;
  MINGW*|MSYS*|CYGWIN*)
    # Windows: require bash, curl, unzip, docker
    if command -v bash >/dev/null 2>&1 && command -v curl >/dev/null 2>&1 && command -v unzip >/dev/null 2>&1 && command -v docker >/dev/null 2>&1; then
      COMPATIBLE=1
    else
      echo "[setup.sh] Windows detected, but required tools (bash, curl, unzip, docker) are missing."
      COMPATIBLE=0
    fi
    ;;
  *)
    echo "[setup.sh] Unsupported OS: $OS_TYPE"
    COMPATIBLE=0
    ;;
esac

if [[ "$COMPATIBLE" -ne 1 ]]; then
  echo "[setup.sh] This OS is not compatible with TokenMin setup requirements."
  exit 1
fi

echo "[setup.sh] Setup mode selected: $MODE (OS: $OS_TYPE)"

case "$MODE" in
  local)
    bash "$SCRIPT_DIR/setup_rust.sh"
    bash "$SCRIPT_DIR/setup_ollama.sh"
    ;;
  lxd|lxc)
    echo "[setup.sh] To verify TokenMin on LXC/LXD (native):"
    echo "1.  lxc launch ubuntu:24.04 tokenmin-tester"
    echo "2.  lxc file push -r . tokenmin-tester/root/"
    echo "3.  lxc exec tokenmin-tester -- bash /root/scripts/setup.sh local"
    ;;
  docker)
    echo "[setup.sh] Building Docker container for verification..."
    docker build -t tokenmin:latest .
    echo "[setup.sh] Docker build complete. To run:"
    echo "docker run -e OLLAMA_URL=http://host.docker.internal:11434 -v /tmp/tokenmin:/tmp tokenmin:latest"
    ;;
  *)
    echo "[setup.sh] Unknown setup mode: $MODE"
    exit 1
    ;;
esac

echo "All dependencies installed (mode: $MODE, OS: $OS_TYPE). Ready to build and run TokenMin."
