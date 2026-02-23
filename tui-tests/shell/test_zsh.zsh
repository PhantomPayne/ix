#!/usr/bin/env zsh
# tui-tests/shell/test_zsh.zsh
# Unit tests for the zsh _ix_widget function using a mock ix binary.

SCRIPT_DIR="${0:a:h}"
export PATH="${SCRIPT_DIR}/mock-ix:${PATH}"

# Define the widget function directly (mirrors what `ix init zsh` produces).
# We test widget BEHAVIOUR here; ix init output is tested by Rust integration tests.
zle() { : }   # stub zle for non-interactive context

_ix_widget() {
    local result
    result=$(ix --pick)
    if [[ $? -eq 0 && -n "$result" ]]; then
        LBUFFER="${LBUFFER}${result} "
    fi
    zle reset-prompt 2>/dev/null || true
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

# Test: widget inserts result into LBUFFER with trailing space
LBUFFER="git diff "
MOCK_IX_OUTPUT="'src/main.rs'" MOCK_IX_EXIT=0 _ix_widget
assert_eq "inserts result at cursor" \
    "git diff 'src/main.rs' " \
    "$LBUFFER"

# Test: widget appends trailing space after insertion
LBUFFER="git diff "
MOCK_IX_OUTPUT="'a.rs'" MOCK_IX_EXIT=0 _ix_widget
assert_eq "appends trailing space" \
    "git diff 'a.rs' " \
    "$LBUFFER"

# Test: widget does not modify LBUFFER on exit 1 (abort)
LBUFFER="git diff "
MOCK_IX_OUTPUT="" MOCK_IX_EXIT=1 _ix_widget
assert_eq "abort: LBUFFER unchanged on exit 1" \
    "git diff " \
    "$LBUFFER"

# Test: widget does not modify LBUFFER on empty output + exit 0
LBUFFER="git diff "
MOCK_IX_OUTPUT="" MOCK_IX_EXIT=0 _ix_widget
assert_eq "abort: LBUFFER unchanged on empty output" \
    "git diff " \
    "$LBUFFER"

# Test: widget inserts into an existing buffer
LBUFFER="git add "
MOCK_IX_OUTPUT="'foo.rs'" MOCK_IX_EXIT=0 _ix_widget
assert_eq "inserts into existing buffer" \
    "git add 'foo.rs' " \
    "$LBUFFER"

# Test: multiple space-separated values inserted as-is
LBUFFER=""
MOCK_IX_OUTPUT="'a.rs' 'b.rs'" MOCK_IX_EXIT=0 _ix_widget
assert_eq "multiple values inserted" \
    "'a.rs' 'b.rs' " \
    "$LBUFFER"

echo ""
echo "zsh: $pass passed, $fail failed"
[[ $fail -eq 0 ]]
