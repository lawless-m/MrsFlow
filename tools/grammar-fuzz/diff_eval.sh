#!/usr/bin/env bash
# Differential evaluator test: lex+parse+evaluate each line of cases_eval.txt
# through the Rust evaluator and the Prolog companion, compare canonical
# Value output. Any divergence is a bug somewhere.
#
# Until eval-1 lands, the evaluator stubs return errors; both sides will
# produce empty stdout on the placeholder case `42`. The harness "passes"
# trivially by emptiness — that becomes a real test once slice-1 produces
# (num 42) from both sides.
set -uo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$HERE/../.." && pwd)"
CASES="$HERE/cases_eval.txt"
LEX_DCG="$HERE/lexical.pl"
SYN_DCG="$HERE/syntactic.pl"
EVAL_DCG="$HERE/evaluator.pl"
UCD="$HERE/unicode_tables.pl"

cd "$REPO_ROOT"
cargo build --quiet --example value_dump

RUST_BIN="$REPO_ROOT/target/debug/examples/value_dump"

pass=0
fail=0

while IFS= read -r line <&3; do
    [ -z "$line" ] && continue
    tmp=$(mktemp /tmp/mrsflow-eval-XXXXXX.m)
    printf '%s' "$line" > "$tmp"

    rust_out=$("$RUST_BIN" "$tmp" 2>/dev/null)
    prolog_out=$(scryer-prolog -f --no-add-history \
        -g "use_module(library(pio)), phrase_from_file(tokens(T), \"$tmp\"), parse(T, A), root_env(E), eval(A, E, V), deep_force(V, Forced), print_value(Forced), nl, halt" \
        "$LEX_DCG" "$UCD" "$SYN_DCG" "$EVAL_DCG" </dev/null 2>/dev/null)

    if [ "$rust_out" = "$prolog_out" ]; then
        pass=$((pass + 1))
    else
        fail=$((fail + 1))
        echo "DIVERGE: $line"
        diff <(printf '%s\n' "$rust_out") <(printf '%s\n' "$prolog_out") | sed 's/^/    /'
    fi
    rm -f "$tmp"
done 3< "$CASES"

echo "---"
echo "passed: $pass    failed: $fail"
[ "$fail" -eq 0 ]
