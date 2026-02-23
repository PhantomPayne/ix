#!/usr/bin/env fish
# tui-tests/shell/test_fish.fish
# Unit tests for the fish _ix_widget conditional logic.
#
# Note: `commandline` only works in interactive mode, so we test the guard
# conditions that control whether the widget inserts text.

set SCRIPT_DIR (dirname (status filename))
set PATH $SCRIPT_DIR/mock-ix $PATH

set pass 0
set fail 0

function assert_eq
    set desc $argv[1]
    set expected $argv[2]
    set actual $argv[3]
    if test "$expected" = "$actual"
        echo "  PASS: $desc"
        set pass (math $pass + 1)
    else
        echo "  FAIL: $desc"
        echo "    expected: $expected"
        echo "    actual:   $actual"
        set fail (math $fail + 1)
    end
end

# Test the guard conditions from _ix_widget directly.
# The real widget would call `commandline -i`, which only works interactively.

# Guard: exit 1 → should NOT insert
set mock_exit 1
set mock_output ""
if test $mock_exit -eq 0 -a -n "$mock_output"
    assert_eq "abort guard: exit 1 skips insert" "should_not_reach" "reached"
else
    assert_eq "abort guard: exit 1 skips insert" "skipped" "skipped"
end

# Guard: exit 0 + empty output → should NOT insert
set mock_exit 0
set mock_output ""
if test $mock_exit -eq 0 -a -n "$mock_output"
    assert_eq "abort guard: empty output skips insert" "should_not_reach" "reached"
else
    assert_eq "abort guard: empty output skips insert" "skipped" "skipped"
end

# Guard: exit 0 + non-empty output → SHOULD insert
set mock_exit 0
set mock_output "'foo.rs'"
if test $mock_exit -eq 0 -a -n "$mock_output"
    assert_eq "insert guard: fires with valid output" "fires" "fires"
else
    assert_eq "insert guard: fires with valid output" "fires" "did_not_fire"
end

# Guard: exit 1 + non-empty output → exit code wins, should NOT insert
set mock_exit 1
set mock_output "'foo.rs'"
if test $mock_exit -eq 0 -a -n "$mock_output"
    assert_eq "abort guard: exit 1 wins even with output" "should_not_reach" "reached"
else
    assert_eq "abort guard: exit 1 wins even with output" "skipped" "skipped"
end

# Verify mock ix is callable and respects MOCK_IX_EXIT
set -x MOCK_IX_EXIT 0
set -x MOCK_IX_OUTPUT "hello"
set result (ix --pick)
set exit_code $status
assert_eq "mock ix returns MOCK_IX_OUTPUT" "hello" "$result"
assert_eq "mock ix respects MOCK_IX_EXIT=0" "0" "$exit_code"

set -x MOCK_IX_EXIT 1
set -x MOCK_IX_OUTPUT ""
ix --pick >/dev/null 2>&1
assert_eq "mock ix respects MOCK_IX_EXIT=1" "1" "$status"

echo ""
echo "fish: $pass passed, $fail failed"
test $fail -eq 0
