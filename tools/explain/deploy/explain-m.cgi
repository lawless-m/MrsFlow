#!/bin/bash
# explain-m.cgi — Apache CGI entry for the M error explainer.
#
# Accepts two request body shapes:
#
#   Content-Type: application/json  ->  { "source": "...", "error": "..." }
#                                        error is optional
#   Content-Type: text/plain        ->  raw M source (no error component)
#
# Flow:
#   1. Read CONTENT_LENGTH bytes of the request body.
#   2. If JSON, split into source + error via jq. Otherwise body == source.
#   3. Pipe source into scryer-prolog on stdin; pass error file path via
#      EXPLAIN_M_ERROR_FILE env var (avoids shell-quoting the user's
#      error string into the -g goal).
#
# Requires scryer >=0.10.0-162 for working stdin EOF on pipes.

set -e

LIB=/usr/local/share/explain-m

printf 'Content-Type: text/plain; charset=utf-8\r\n'
printf '\r\n'

LEN="${CONTENT_LENGTH:-0}"
if [ "$LEN" -gt 1048576 ]; then
    echo "Request too large (max 1 MB)."
    exit 0
fi

REQDIR=$(mktemp -d /tmp/explain-m-req.XXXXXX)
trap 'rm -rf "$REQDIR"' EXIT

BODY="$REQDIR/body"
SRC="$REQDIR/src.m"
ERR="$REQDIR/err.txt"

head -c "$LEN" > "$BODY"

CT="${CONTENT_TYPE:-text/plain}"
case "$CT" in
    application/json*)
        if ! jq -er '.source // ""' < "$BODY" > "$SRC" 2>/dev/null; then
            echo "Request JSON is invalid or missing a 'source' field."
            exit 0
        fi
        jq -r '.error // ""' < "$BODY" > "$ERR" 2>/dev/null || : > "$ERR"
        ;;
    *)
        cp "$BODY" "$SRC"
        : > "$ERR"
        ;;
esac

# Hand the error path to scryer via env if non-empty; the Prolog side
# checks the var.
if [ -s "$ERR" ]; then
    export EXPLAIN_M_ERROR_FILE="$ERR"
fi

exec /usr/local/bin/scryer-prolog -f --no-add-history \
    -g 'cgi_main, halt' \
    "$LIB/lexical.pl" \
    "$LIB/unicode_tables.pl" \
    "$LIB/error_rules.pl" \
    "$LIB/cgi.pl" \
    < "$SRC"
