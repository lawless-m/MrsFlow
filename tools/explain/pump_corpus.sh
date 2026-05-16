#!/usr/bin/env bash
# pump_corpus.sh — run the explainer over a directory of .m files and
# print a one-line summary per file: filename | rule-id-or-blank | first-line.
#
# Used to flush out false positives (rule fires on legitimate source).
#
# Usage:
#   tools/explain/pump_corpus.sh examples/powerqueries
#   tools/explain/pump_corpus.sh Oracle/cases

set -uo pipefail

if [ $# -ne 1 ]; then
    echo "usage: $0 <directory-of-.m-files>" >&2
    exit 1
fi

HERE="$(cd "$(dirname "$0")" && pwd)"
REPO="$(cd "$HERE/../.." && pwd)"
DIR="$1"

shopt -s nullglob
files=("$DIR"/*.m)
shopt -u nullglob

if [ ${#files[@]} -eq 0 ]; then
    echo "no .m files in $DIR" >&2
    exit 1
fi

matched=0
clean=0
errored=0

for f in "${files[@]}"; do
    name=$(basename "$f")
    if command -v cygpath >/dev/null 2>&1; then
        src=$(cygpath -m "$f")
    else
        src="$f"
    fi
    out=$(scryer-prolog -f --no-add-history \
        -g "explain_file(\"$src\"), halt" \
        "$REPO/tools/grammar-fuzz/lexical.pl" \
        "$REPO/tools/grammar-fuzz/unicode_tables.pl" \
        "$REPO/tools/explain/error_rules.pl" \
        "$REPO/tools/explain/explain.pl" \
        2>&1)
    first=$(printf '%s\n' "$out" | head -1)
    case "$first" in
        \[*\]*)
            rule=$(printf '%s' "$first" | sed -E 's/^\[([^]]+)\].*/\1/')
            printf '%-30s  MATCH  %s\n' "$name" "$rule"
            matched=$((matched + 1))
            ;;
        "No known-mistake"*)
            clean=$((clean + 1))
            ;;
        "Lex error"*)
            printf '%-30s  LEX    -\n' "$name"
            errored=$((errored + 1))
            ;;
        *)
            printf '%-30s  ???    %s\n' "$name" "$first"
            errored=$((errored + 1))
            ;;
    esac
done

echo "---"
printf 'matched: %d   clean: %d   errored: %d\n' "$matched" "$clean" "$errored"
