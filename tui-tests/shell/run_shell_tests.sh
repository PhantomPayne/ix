#!/bin/bash
# tui-tests/shell/run_shell_tests.sh
# Run all shell widget unit tests. Skips shells that are not installed.

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
pass=0
fail=0

run_test() {
    local name="$1"
    local cmd="$2"
    echo "=== $name ==="
    if eval "$cmd"; then
        pass=$((pass + 1))
    else
        fail=$((fail + 1))
    fi
    echo ""
}

if command -v zsh &>/dev/null; then
    run_test "zsh widget" "zsh ${SCRIPT_DIR}/test_zsh.zsh"
else
    echo "=== zsh: skipped (not installed) ==="
    echo ""
fi

run_test "bash widget" "bash ${SCRIPT_DIR}/test_bash.bash"

if command -v fish &>/dev/null; then
    run_test "fish widget" "fish ${SCRIPT_DIR}/test_fish.fish"
else
    echo "=== fish: skipped (not installed) ==="
    echo ""
fi

echo "Shell tests: ${pass} shell(s) passed, ${fail} failed"
[ "$fail" -eq 0 ]
