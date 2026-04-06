#!/bin/bash

set -euo pipefail

IMAGE="${1:-rcore-docker}"
MODE="${2:-all}"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"

docker run --rm \
    -v "${ROOT}:/mnt" \
    -w /mnt \
    "${IMAGE}" \
    bash -lc "bash test.sh ${MODE}"
