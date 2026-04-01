#!/bin/bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
OUT_DIR="${T8_OUT_DIR:-$ROOT/target/t8}"
TOOLCHAIN="${RUSTUP_TOOLCHAIN:-stable}"
export TG_USER_DIR="${TG_USER_DIR:-$ROOT/tg-rcore-tutorial-user}"

mkdir -p "$OUT_DIR"

assert_pattern() {
    local file="$1"
    local pattern="$2"
    local desc="$3"
    if command -v rg >/dev/null 2>&1; then
        rg -F -q "$pattern" "$file"
    else
        grep -F -q "$pattern" "$file"
    fi
    if [ $? -eq 0 ]; then
        echo "[PASS] $desc"
    else
        echo "[FAIL] $desc"
        echo "  pattern: $pattern"
        echo "  log: $file"
        return 1
    fi
}

run_chapter() {
    local crate="$1"
    local feature_flag="$2"
    local log_file="$OUT_DIR/${crate}.log"
    local chapter_env=()

    if [ "$crate" = "tg-rcore-tutorial-ch8" ]; then
        chapter_env=(CHAPTER=t8-8)
    fi

    echo "==> running ${crate} ${feature_flag}"
    (
        cd "$ROOT/$crate"
        env "${chapter_env[@]}" cargo +"$TOOLCHAIN" run $feature_flag 2>&1
    ) | tee "$log_file"
    echo "==> saved log to $log_file"
}

check_ch3() {
    local file="$OUT_DIR/tg-rcore-tutorial-ch3.log"
    assert_pattern "$file" "T8 ch3 observe OK!" "ch3 user workload completed"
    assert_pattern "$file" "[EVENT][ch3][schedule]" "ch3 schedule event emitted"
    assert_pattern "$file" "[EVENT][ch3][syscall]" "ch3 syscall event emitted"
    assert_pattern "$file" "[METRIC][ch3][timer_events]" "ch3 timer metric emitted"
}

check_ch4() {
    local file="$OUT_DIR/tg-rcore-tutorial-ch4.log"
    assert_pattern "$file" "T8 ch4 vm observe OK!" "ch4 user workload completed"
    assert_pattern "$file" "[EVENT][ch4][syscall]" "ch4 syscall event emitted"
    assert_pattern "$file" "[EVENT][ch4][vm]" "ch4 vm fault event emitted"
    assert_pattern "$file" "[METRIC][ch4][vm_fault_events]" "ch4 vm metric emitted"
}

check_ch8() {
    local file="$OUT_DIR/tg-rcore-tutorial-ch8.log"
    assert_pattern "$file" "T8 ch8 Usertests passed!" "ch8 user workload completed"
    assert_pattern "$file" "[EVENT][ch8][sync] mutex_lock" "ch8 mutex event emitted"
    assert_pattern "$file" "[EVENT][ch8][sync] sem_down" "ch8 semaphore event emitted"
    assert_pattern "$file" "[EVENT][ch8][sync] condvar_wait" "ch8 condvar event emitted"
    assert_pattern "$file" "[EVENT][ch8][sync] deadlock-detect enabled=true" "ch8 deadlock toggle emitted"
    assert_pattern "$file" "[METRIC][ch8][sync_blocked_events]" "ch8 blocked metric emitted"
}

run_target() {
    local target="$1"
    case "$target" in
        ch3)
            run_chapter "tg-rcore-tutorial-ch3" "--features exercise"
            check_ch3
            ;;
        ch4)
            run_chapter "tg-rcore-tutorial-ch4" "--features exercise"
            check_ch4
            ;;
        ch8)
            run_chapter "tg-rcore-tutorial-ch8" "--features exercise"
            check_ch8
            ;;
        all)
            run_target ch3
            run_target ch4
            run_target ch8
            ;;
        *)
            echo "usage: $0 [ch3|ch4|ch8|all]"
            return 1
            ;;
    esac
}

run_target "${1:-all}"
