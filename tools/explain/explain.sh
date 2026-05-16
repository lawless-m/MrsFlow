#!/usr/bin/env bash
# explain.sh — run the M error explainer on a source file (Windows-friendly).
#
# Usage:  tools/explain/explain.sh path/to/source.m [path/to/error.txt]
#
# Uses the file-path entry point (`explain_file_with_error/2` in
# explain.pl) rather than the stdin path the production CGI uses.
# Reason: scryer-prolog's `phrase_from_stream(user_input)` hangs on
# pipes from git-bash on Windows even after the 0.10.0-162 fix, so
# the local CLI sticks to file reads which work cross-platform.
#
# Both entry points share the same matching logic in error_rules.pl,
# so the diagnostic output is identical to what the CGI returns.

set -euo pipefail

if [ $# -lt 1 ] || [ $# -gt 2 ]; then
    echo "usage: $0 <source.m> [<error.txt>]" >&2
    exit 1
fi

HERE="$(cd "$(dirname "$0")" && pwd)"
REPO="$(cd "$HERE/../.." && pwd)"

to_native() {
    if command -v cygpath >/dev/null 2>&1; then
        cygpath -m "$1"
    else
        echo "$1"
    fi
}

SRC=$(to_native "$1")

if [ $# -eq 2 ]; then
    ERR=$(to_native "$2")
    GOAL="explain_file_with_error(\"$SRC\", file(\"$ERR\")), halt"
else
    GOAL="explain_file(\"$SRC\"), halt"
fi

scryer-prolog -f --no-add-history \
    -g "$GOAL" \
    "$REPO/tools/grammar-fuzz/lexical.pl" \
    "$REPO/tools/grammar-fuzz/unicode_tables.pl" \
    "$REPO/tools/explain/error_rules.pl" \
    "$REPO/tools/explain/explain.pl"
