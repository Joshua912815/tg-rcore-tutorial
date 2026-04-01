#!/bin/bash

OUTPUT=$(printf 'q' | cargo run 2>&1)

if echo "$OUTPUT" | grep -q "Tangram rendered"; then
    echo "Test PASSED: Tangram framebuffer rendered and exited cleanly"
    exit 0
else
    echo "Test FAILED: Tangram render log not found"
    echo "Actual output:"
    echo "$OUTPUT"
    exit 1
fi
