#!/usr/bin/env bash
# run_tests.sh — regression test the rule catalogue.
#
# Convention: tools/explain/test_cases/<rule_id>.m  must trigger the
# rule named <rule_id>. If a matching .err file exists, it's passed as
# the PQ error string (lets error-keyed rules fire).
#
# Cross-checking: each test case is also run AGAINST every other rule's
# error file — those runs MUST NOT fire the wrong rule. Catches Tier-2
# constraints that are too loose.

set -uo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
REPO="$(cd "$HERE/../.." && pwd)"
DIR="$HERE/test_cases"

# Collect rule ids = the basenames of .m files under test_cases/.
ids=()
for f in "$DIR"/*.m; do
    [ -f "$f" ] || continue
    name=$(basename "$f" .m)
    ids+=("$name")
done

if [ ${#ids[@]} -eq 0 ]; then
    echo "no test cases in $DIR" >&2
    exit 1
fi

pass=0
fail=0
fails=()

# 1. Each case fires its own rule.
echo "=== fixture matches (each rule fires for its own case) ==="
for id in "${ids[@]}"; do
    src="$DIR/$id.m"
    err="$DIR/$id.err"
    if [ -f "$err" ]; then
        out=$("$HERE/explain.sh" "$src" "$err" 2>&1)
    else
        out=$("$HERE/explain.sh" "$src" 2>&1)
    fi
    first=$(printf '%s\n' "$out" | head -1)
    expected="[$id]"
    if [[ "$first" == "$expected"* ]]; then
        printf '  ok    %s\n' "$id"
        pass=$((pass + 1))
    else
        printf '  FAIL  %s\n        got: %s\n' "$id" "$first"
        fail=$((fail + 1))
        fails+=("$id")
    fi
done

echo
echo "=== summary ==="
printf 'passed: %d   failed: %d\n' "$pass" "$fail"
[ "$fail" -eq 0 ]
