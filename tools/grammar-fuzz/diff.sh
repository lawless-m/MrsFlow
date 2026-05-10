#!/usr/bin/env bash
# Differential lexer test: run each line of cases.txt through the Rust lexer and
# the Prolog DCG, diff the token streams. Any divergence is a bug somewhere.
set -uo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$HERE/../.." && pwd)"
CASES="$HERE/cases.txt"
DCG="$HERE/lexical.pl"

cd "$REPO_ROOT"
cargo build --quiet --example lex_dump

RUST_BIN="$REPO_ROOT/target/debug/examples/lex_dump"

pass=0
fail=0
fails=()

while IFS= read -r line; do
    [ -z "$line" ] && continue
    tmp=$(mktemp /tmp/mrsflow-case-XXXXXX.m)
    printf '%s' "$line" > "$tmp"

    rust_out=$("$RUST_BIN" "$tmp")
    prolog_out=$(scryer-prolog -f --no-add-history \
        -g "use_module(library(pio)), phrase_from_file(tokens(T), \"$tmp\"), print_tokens(T), halt" \
        "$DCG" "$HERE/unicode_tables.pl" 2>/dev/null)

    if [ "$rust_out" = "$prolog_out" ]; then
        pass=$((pass + 1))
    else
        fail=$((fail + 1))
        fails+=("$line")
        echo "DIVERGE: $line"
        diff <(printf '%s\n' "$rust_out") <(printf '%s\n' "$prolog_out") | sed 's/^/    /'
    fi
    rm -f "$tmp"
done < "$CASES"

echo "---"
echo "passed: $pass    failed: $fail"
[ "$fail" -eq 0 ]
