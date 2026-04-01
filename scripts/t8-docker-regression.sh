#!/bin/bash
set -euo pipefail

IMAGE="${1:-rcore-docker}"
TARGET="${2:-all}"
CONTAINER="t8-docker-regression"

cleanup() {
    docker rm -f "$CONTAINER" >/dev/null 2>&1 || true
}
trap cleanup EXIT

docker run -d --name "$CONTAINER" -v "$PWD":/mnt -w /mnt "$IMAGE" bash -lc 'sleep infinity' >/dev/null

docker exec "$CONTAINER" bash -lc '
set -e
rustup toolchain install stable-aarch64-unknown-linux-gnu --profile minimal
rustup target add riscv64gc-unknown-none-elf --toolchain stable-aarch64-unknown-linux-gnu
rustup component add rust-src llvm-tools-preview rustfmt clippy --toolchain stable-aarch64-unknown-linux-gnu
export RUSTUP_TOOLCHAIN=stable-aarch64-unknown-linux-gnu
export TG_USER_DIR=/mnt/tg-rcore-tutorial-user
/mnt/scripts/t8-regression.sh '"$TARGET"'
'
