#!/bin/bash

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
IMAGE="${1:-rcore-docker}"
MODE="${2:-all}"
CONTAINER="my-rcore-t21"

if ! docker ps -a --format '{{.Names}}' | grep -qx "${CONTAINER}"; then
    docker run -d \
        --name "${CONTAINER}" \
        -v "${ROOT}:/mnt" \
        -w /mnt \
        "${IMAGE}" \
        bash -lc 'sleep infinity' >/dev/null
elif ! docker ps --format '{{.Names}}' | grep -qx "${CONTAINER}"; then
    docker start "${CONTAINER}" >/dev/null
fi

docker exec "${CONTAINER}" bash -lc "cd /mnt/tg-rcore-tutorial-ch3-observe && bash test.sh ${MODE}"
