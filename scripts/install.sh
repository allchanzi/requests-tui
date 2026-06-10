#!/usr/bin/env bash
# Download the latest requests-tui release binary for this platform and install it.
# Usage:
#   ./install.sh                 # install latest into ~/.local/bin
#   BINDIR=/usr/local/bin ./install.sh
#   VERSION=v0.1.0 ./install.sh  # pin a specific release tag
set -euo pipefail

REPO="${REPO:-allchanzi/requests-tui}"     # change if your repo lives elsewhere
BINDIR="${BINDIR:-$HOME/.local/bin}"
VERSION="${VERSION:-latest}"

case "$(uname -s)-$(uname -m)" in
  Darwin-arm64)   target="aarch64-apple-darwin" ;;
  Darwin-x86_64)  target="x86_64-apple-darwin" ;;
  Linux-x86_64)   target="x86_64-unknown-linux-gnu" ;;
  Linux-aarch64)  target="aarch64-unknown-linux-gnu" ;;
  *) echo "unsupported platform: $(uname -s)-$(uname -m)" >&2; exit 1 ;;
esac

asset="requests-tui-${target}.tar.gz"
if [ "$VERSION" = "latest" ]; then
  url="https://github.com/${REPO}/releases/latest/download/${asset}"
else
  url="https://github.com/${REPO}/releases/download/${VERSION}/${asset}"
fi

echo "Installing requests-tui (${target}) from ${url}"
mkdir -p "$BINDIR"
curl -fsSL "$url" | tar -xz -C "$BINDIR"
echo "Installed: ${BINDIR}/requests-tui"
echo "Ensure ${BINDIR} is on your PATH."
