#!/bin/bash

set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(cd -- "$SCRIPT_DIR/.." && pwd)"
ARCHIVE="$ROOT_DIR/bundled/vendor-src.tar.gz"

if [ -f "$ROOT_DIR/vendor/tg-rcore-tutorial-user/Cargo.toml" ] && \
   [ -f "$ROOT_DIR/vendor/tg-rcore-tutorial-easy-fs/Cargo.toml" ] && \
   [ -f "$ROOT_DIR/vendor/tg-rcore-tutorial-checker/Cargo.toml" ]; then
    exit 0
fi

if [ ! -f "$ARCHIVE" ]; then
    echo "missing bundled vendor archive: $ARCHIVE" >&2
    exit 1
fi

rm -rf "$ROOT_DIR/vendor"
tar -xzf "$ARCHIVE" -C "$ROOT_DIR"
