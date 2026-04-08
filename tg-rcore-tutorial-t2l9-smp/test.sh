#!/bin/bash

set -euo pipefail

echo "[test] running ch1 SMP binary"
OUTPUT_CH1=$(cargo run --bin t2l9-ch1-smp 2>&1)
echo "$OUTPUT_CH1"
echo "$OUTPUT_CH1" | grep -q "all 4 harts reached S-mode"
echo "$OUTPUT_CH1" | grep -q "hart "

echo "[test] running ch2 SMP binary"
OUTPUT_CH2=$(cargo run --bin t2l9-ch2-smp 2>&1)
echo "$OUTPUT_CH2"
echo "$OUTPUT_CH2" | grep -q "secondary hart"
echo "$OUTPUT_CH2" | grep -q "Hello, world from user mode program!"
echo "$OUTPUT_CH2" | grep -q "Test power_3 OK!"
echo "$OUTPUT_CH2" | grep -q "Test power_5 OK!"
echo "$OUTPUT_CH2" | grep -q "Test power_7 OK!"

echo "[test] all checks passed"
