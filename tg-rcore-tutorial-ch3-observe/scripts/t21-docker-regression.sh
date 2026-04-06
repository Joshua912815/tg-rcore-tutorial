#!/bin/bash

set -euo pipefail

IMAGE="${1:-rcore-docker}"
MODE="${2:-all}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
CONTAINER="t21-docker-regression"

cleanup() {
    docker rm -f "$CONTAINER" >/dev/null 2>&1 || true
}
trap cleanup EXIT

docker run -d --name "$CONTAINER" -v "${ROOT}:/mnt" -w /mnt "$IMAGE" bash -lc 'sleep infinity' >/dev/null

docker exec "$CONTAINER" bash -lc '
set -e
rustup toolchain install stable-aarch64-unknown-linux-gnu --profile minimal
rustup target add riscv64gc-unknown-none-elf --toolchain stable-aarch64-unknown-linux-gnu
rustup component add rust-src llvm-tools-preview rustfmt clippy --toolchain stable-aarch64-unknown-linux-gnu
export RUSTUP_TOOLCHAIN=stable-aarch64-unknown-linux-gnu
export RUN_TIMEOUT=300s
bash test.sh '"$MODE"'
'
