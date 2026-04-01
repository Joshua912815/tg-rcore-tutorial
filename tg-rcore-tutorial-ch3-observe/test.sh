#!/bin/bash

set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
OUT_DIR="${ROOT}/../target/t21"
mkdir -p "${OUT_DIR}"

if command -v rg >/dev/null 2>&1; then
    FIND="rg -n"
else
    FIND="grep -n"
fi

run_with_timeout() {
    if command -v timeout >/dev/null 2>&1; then
        timeout "$@"
    elif command -v gtimeout >/dev/null 2>&1; then
        gtimeout "$@"
    else
        shift
        "$@"
    fi
}

check_contains() {
    local file="$1"
    local pattern="$2"
    local label="$3"
    if ${FIND} "$pattern" "$file" >/dev/null 2>&1; then
        echo "[PASS] ${label}"
    else
        echo "[FAIL] ${label}"
        echo "log file: ${file}"
        exit 1
    fi
}

run_base() {
    local log_file="${OUT_DIR}/ch3-observe-base.log"
    run_with_timeout 30s cargo run >"${log_file}" 2>&1
    check_contains "${log_file}" "T21 ch3 normal OK!" "normal workload completed"
    check_contains "${log_file}" "T21 ch3 breakpoint resumed" "breakpoint workload resumed"
    check_contains "${log_file}" "\\[OBS\\]\\[breakpoint\\].*step=" "breakpoint step recorded"
    check_contains "${log_file}" "\\[OBS\\]\\[exception\\].*store-fault" "exception path recorded"
    check_contains "${log_file}" "Test sleep OK!" "baseline sleep workload completed"
    check_contains "${log_file}" "\\[OBS\\]\\[metric\\] reason=summary" "summary metric emitted"
    echo "base log: ${log_file}"
}

run_crash() {
    local log_file="${OUT_DIR}/ch3-observe-crash.log"
    run_with_timeout 30s cargo run --features crash-demo >"${log_file}" 2>&1
    check_contains "${log_file}" "T21 crash trigger" "crash workload started"
    check_contains "${log_file}" "\\[OBS\\]\\[snapshot\\] reason=trace-crash" "crash snapshot emitted"
    check_contains "${log_file}" "\\[OBS\\]\\[panic\\] breadcrumb_depth=" "panic breadcrumb header emitted"
    check_contains "${log_file}" "\\[OBS\\]\\[crumb\\] #3 panic_stage2" "panic breadcrumb tail emitted"
    check_contains "${log_file}" "T21 crash demo task=" "panic message emitted"
    echo "crash log: ${log_file}"
}

case "${1:-all}" in
    base)
        run_base
        ;;
    crash)
        run_crash
        ;;
    all)
        run_base
        run_crash
        ;;
    *)
        echo "usage: $0 [base|crash|all]"
        exit 1
        ;;
esac
