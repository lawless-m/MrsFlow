#!/usr/bin/env bash
# Differential parser test: lex+parse each line of cases_parser.txt through the
# Rust parser and the Prolog DCG, compare canonical S-expressions. Any
# divergence is a bug somewhere.
set -uo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$HERE/../.." && pwd)"
CASES="$HERE/cases_parser.txt"
LEX_DCG="$HERE/lexical.pl"
SYN_DCG="$HERE/syntactic.pl"
UCD="$HERE/unicode_tables.pl"

cd "$REPO_ROOT"
cargo build --quiet --example ast_dump

RUST_BIN="$REPO_ROOT/target/debug/examples/ast_dump"

pass=0
fail=0

while IFS= read -r line; do
    [ -z "$line" ] && continue
    tmp=$(mktemp /tmp/mrsflow-pcase-XXXXXX.m)
    printf '%s' "$line" > "$tmp"

    rust_out=$("$RUST_BIN" "$tmp")
    prolog_out=$(scryer-prolog -f --no-add-history \
        -g "use_module(library(pio)), phrase_from_file(tokens(T), \"$tmp\"), parse(T, A), print_ast(A), halt" \
        "$LEX_DCG" "$UCD" "$SYN_DCG" 2>/dev/null)

    if [ "$rust_out" = "$prolog_out" ]; then
        pass=$((pass + 1))
    else
        fail=$((fail + 1))
        echo "DIVERGE: $line"
        diff <(printf '%s\n' "$rust_out") <(printf '%s\n' "$prolog_out") | sed 's/^/    /'
    fi
    rm -f "$tmp"
done < "$CASES"

echo "---"
echo "passed: $pass    failed: $fail"
[ "$fail" -eq 0 ]
