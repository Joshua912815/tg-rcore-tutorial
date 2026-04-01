#!/bin/bash
set -euo pipefail

IMAGE="${1:-rcore-docker}"
CONTAINER="t8-docker-check"

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

cd /mnt/tg-rcore-tutorial-console
cargo +stable check

cd /mnt/tg-rcore-tutorial-checker
cargo +stable check

cd /mnt/tg-rcore-tutorial-ch3
TG_SKIP_USER_APPS=1 cargo +stable check

cd /mnt/tg-rcore-tutorial-ch4
TG_SKIP_USER_APPS=1 cargo +stable check

cd /mnt/tg-rcore-tutorial-ch8
TG_SKIP_USER_APPS=1 cargo +stable check
'

echo "T8 Docker checks completed successfully."
