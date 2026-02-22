#!/bin/sh
# install.sh — install ix from GitHub Releases
# Usage: curl -fsSL https://raw.githubusercontent.com/PhantomPayne/ix/main/install.sh | sh

set -e

REPO="https://github.com/PhantomPayne/ix"
BIN_DIR="${HOME}/.local/bin"
mkdir -p "$BIN_DIR"

# detect platform
OS=$(uname -s)
ARCH=$(uname -m)

case "$OS-$ARCH" in
  Linux-x86_64)  TARGET="ix-linux-x86_64" ;;
  Linux-aarch64) TARGET="ix-linux-aarch64" ;;
  Darwin-x86_64) TARGET="ix-macos-x86_64" ;;
  Darwin-arm64)  TARGET="ix-macos-aarch64" ;;
  *)
    echo "Unsupported platform: $OS-$ARCH" >&2
    exit 1
    ;;
esac

# resolve latest version tag
VERSION=$(curl -fsSL "https://api.github.com/repos/PhantomPayne/ix/releases/latest" \
  | grep '"tag_name"' \
  | sed 's/.*"tag_name": "\(.*\)".*/\1/')

if [ -z "$VERSION" ]; then
  echo "Could not determine latest release version" >&2
  exit 1
fi

URL="${REPO}/releases/download/${VERSION}/${TARGET}"

echo "Installing ix ${VERSION} for ${OS}-${ARCH}..."
curl -fsSL "$URL" -o "$BIN_DIR/ix"
chmod +x "$BIN_DIR/ix"

echo "Installed to $BIN_DIR/ix"

# warn if BIN_DIR is not in PATH
case ":$PATH:" in
  *":$BIN_DIR:"*) ;;
  *) echo "Note: add $BIN_DIR to your PATH (e.g. export PATH=\"\$HOME/.local/bin:\$PATH\")" ;;
esac
