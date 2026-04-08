#!/bin/bash

set -euo pipefail

ROOT_DIR=$(cd "$(dirname "$0")" && pwd)
TARGET_DIR="$ROOT_DIR/target/riscv64gc-unknown-none-elf/debug"
QEMU=qemu-system-riscv64

run_kernel() {
    local smp=$1
    local bin=$2
    "$QEMU" -machine virt -smp "$smp" -nographic -bios default -kernel "$TARGET_DIR/$bin" 2>&1
}

echo "[observe] building binaries"
cargo build --bin t2l9-ch1-smp --bin t2l9-ch2-smp >/dev/null

echo
echo "[observe] ch1 multicore startup matrix"
for smp in 1 2 4; do
    output=$(run_kernel "$smp" t2l9-ch1-smp)
    platform=$(printf '%s\n' "$output" | grep -oE 'Platform HART Count +: [0-9]+' | tail -n1 | awk '{print $NF}')
    online=$(printf '%s\n' "$output" | grep -oE 'hart [0-9]+ online' | wc -l | tr -d ' ')
    summary=$(printf '%s\n' "$output" | grep -oE 'all [0-9]+ harts reached S-mode' | tail -n1 || true)
    printf '  - smp=%s: platform_harts=%s, online_messages=%s, summary="%s"\n' \
        "$smp" "${platform:-unknown}" "$online" "${summary:-missing due to interleaved output}"
done

echo
echo "[observe] ch2 startup/parking matrix"
for smp in 1 2 4; do
    output=$(run_kernel "$smp" t2l9-ch2-smp)
    platform=$(printf '%s\n' "$output" | grep -oE 'Platform HART Count +: [0-9]+' | tail -n1 | awk '{print $NF}')
    detected=$(printf '%s\n' "$output" | grep -oE 'detected [0-9]+ harts' | tail -n1 || true)
    parked=$(printf '%s\n' "$output" | grep -oE '\[T2L9/ch2\] secondary hart [0-9]+' | wc -l | tr -d ' ' || true)
    app_hello=$(printf '%s\n' "$output" | grep -c 'Hello, world from user mode program!' || true)
    app_power=$(printf '%s\n' "$output" | grep -c 'Test power_' || true)
    printf '  - smp=%s: platform_harts=%s, %s, parked=%s, app_hello=%s, power_tests=%s\n' \
        "$smp" "${platform:-unknown}" "${detected:-detected line missing due to interleaved output}" "$parked" "$app_hello" "$app_power"
done
