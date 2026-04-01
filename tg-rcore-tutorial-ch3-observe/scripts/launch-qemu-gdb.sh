#!/bin/bash

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
PROFILE="${PROFILE:-debug}"

cd "${ROOT}"

if [ "${PROFILE}" = "release" ]; then
    cargo build --release
    ELF="target/riscv64gc-unknown-none-elf/release/tg-rcore-tutorial-ch3-observe"
else
    cargo build
    ELF="target/riscv64gc-unknown-none-elf/debug/tg-rcore-tutorial-ch3-observe"
fi

echo "launching qemu with GDB stub on :1234"
exec qemu-system-riscv64 \
    -machine virt \
    -nographic \
    -bios none \
    -kernel "${ELF}" \
    -s -S
