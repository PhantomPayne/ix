#!/usr/bin/env bash
# tui-tests/shell/test_bash.bash
# Unit tests for the bash _ix_widget function using a mock ix binary.
#
# The mock-ix/ix executable is put on PATH first so _ix_widget calls our mock
# instead of the real ix binary.

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
export PATH="${SCRIPT_DIR}/mock-ix:${PATH}"

# Define the widget function directly (mirrors what `ix init bash` produces).
# We test widget BEHAVIOUR here; ix init output is tested by Rust integration tests.
_ix_widget() {
    local result
    result=$(ix --pick)
    if [[ $? -eq 0 && -n "$result" ]]; then
        READLINE_LINE="${READLINE_LINE:0:$READLINE_POINT}${result} ${READLINE_LINE:$READLINE_POINT}"
        READLINE_POINT=$(( READLINE_POINT + ${#result} + 1 ))
    fi
}

pass=0
fail=0

assert_eq() {
    local desc="$1" expected="$2" actual="$3"
    if [[ "$expected" == "$actual" ]]; then
        echo "  PASS: $desc"
        (( pass++ ))
    else
        echo "  FAIL: $desc"
        echo "    expected: $(printf '%q' "$expected")"
        echo "    actual:   $(printf '%q' "$actual")"
        (( fail++ ))
    fi
}

# Test: widget inserts result with trailing space
READLINE_LINE="git diff "
READLINE_POINT=${#READLINE_LINE}
MOCK_IX_OUTPUT="'src/main.rs'" MOCK_IX_EXIT=0 _ix_widget
assert_eq "inserts result with trailing space" \
    "git diff 'src/main.rs' " \
    "$READLINE_LINE"

# Test: abort on exit 1 — READLINE_LINE unchanged
READLINE_LINE="git diff "
READLINE_POINT=${#READLINE_LINE}
MOCK_IX_OUTPUT="" MOCK_IX_EXIT=1 _ix_widget
assert_eq "abort: unchanged on exit 1" \
    "git diff " \
    "$READLINE_LINE"

# Test: abort on empty output + exit 0
READLINE_LINE="git diff "
READLINE_POINT=${#READLINE_LINE}
MOCK_IX_OUTPUT="" MOCK_IX_EXIT=0 _ix_widget
assert_eq "abort: unchanged on empty output" \
    "git diff " \
    "$READLINE_LINE"

# Test: cursor position advances past inserted text
READLINE_LINE="git diff "
READLINE_POINT=${#READLINE_LINE}
result="'foo.rs'"
MOCK_IX_OUTPUT="$result" MOCK_IX_EXIT=0 _ix_widget
prefix="git diff "
expected_point=$(( ${#prefix} + ${#result} + 1 ))
assert_eq "cursor advances past insertion" \
    "$expected_point" \
    "$READLINE_POINT"

# Test: insertion at cursor in the middle of a line
READLINE_LINE="git diff  --stat"
READLINE_POINT=9   # position after "git diff "
MOCK_IX_OUTPUT="'bar.rs'" MOCK_IX_EXIT=0 _ix_widget
assert_eq "insertion at middle of line" \
    "git diff 'bar.rs'  --stat" \
    "$READLINE_LINE"

# Test: multiple values inserted as-is
READLINE_LINE=""
READLINE_POINT=0
MOCK_IX_OUTPUT="'a.rs' 'b.rs'" MOCK_IX_EXIT=0 _ix_widget
assert_eq "multiple values inserted" \
    "'a.rs' 'b.rs' " \
    "$READLINE_LINE"

echo ""
echo "bash: $pass passed, $fail failed"
[[ $fail -eq 0 ]]
