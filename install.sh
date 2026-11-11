#!/bin/sh
# costcrew installer — curl -fsSL https://raw.githubusercontent.com/Arch087146/costcrew/main/install.sh | sh
set -eu

REPO="Arch087146/costcrew"
VERSION="${COSTCREW_VERSION:-latest}"

# pick the asset for this machine
OS="$(uname -s)"
ARCH="$(uname -m)"
case "$OS" in
    Darwin) OS_TAG="macos" ;;
    Linux) OS_TAG="linux" ;;
    *) echo "unsupported OS: $OS (windows users: use install.ps1)" >&2; exit 1 ;;
esac
case "$ARCH" in
    arm64 | aarch64) ARCH_TAG="aarch64" ;;
    x86_64 | amd64) ARCH_TAG="x86_64" ;;
    *) echo "unsupported arch: $ARCH" >&2; exit 1 ;;
esac
ASSET="costcrew-${OS_TAG}-${ARCH_TAG}.tar.gz"

if [ "$VERSION" = "latest" ]; then
    URL="https://github.com/${REPO}/releases/latest/download/${ASSET}"
else
    URL="https://github.com/${REPO}/releases/download/${VERSION}/${ASSET}"
fi

# install destination: writable dir on PATH, else ~/.local/bin
if [ -n "${COSTCREW_INSTALL_DIR:-}" ]; then
    DEST="$COSTCREW_INSTALL_DIR"
elif [ -w /usr/local/bin ] 2>/dev/null; then
    DEST="/usr/local/bin"
else
    DEST="${HOME}/.local/bin"
    mkdir -p "$DEST"
    case ":$PATH:" in
        *":$DEST:"*) ;;
        *) echo "note: add $DEST to your PATH:  export PATH=\"\$HOME/.local/bin:\$PATH\"" ;;
    esac
fi

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

echo "fetching ${ASSET} ..."
curl -fsSL "$URL" -o "$TMP/costcrew.tar.gz"
tar xzf "$TMP/costcrew.tar.gz" -C "$TMP"

mv "$TMP/costcrew" "$DEST/costcrew"
chmod +x "$DEST/costcrew"

echo
echo "installed: $(command -v costcrew 2>/dev/null || echo "$DEST/costcrew")"
"$DEST/costcrew" --version
echo "run  costcrew  to open the app · costcrew prices · costcrew spend"
