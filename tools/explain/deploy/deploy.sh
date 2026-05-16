#!/usr/bin/env bash
# deploy.sh — package and ship the explainer to vsprod.
#
# Builds a tarball of the four library .pl files + the cgi script +
# apache conf, streams it over ssh into the server-side install.sh.

set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
REPO="$(cd "$HERE/../../.." && pwd)"
HOST="${HOST:-vsprod}"

cd "$REPO"

# Two steps because install.sh takes the tarball as a positional arg —
# we can't fuse stdin (the script) with the tarball into one ssh.
REMOTE_TGZ="/tmp/explain-m.$$.tgz"
tar -czf - \
    -C tools/grammar-fuzz lexical.pl unicode_tables.pl \
    -C "$REPO/tools/explain" error_rules.pl explain.pl cgi.pl \
    -C "$REPO/tools/explain/deploy" explain-m.cgi explain-m.conf \
    | ssh "$HOST" "cat > $REMOTE_TGZ"
ssh "$HOST" "sudo bash -s $REMOTE_TGZ" < "$HERE/install.sh"
ssh "$HOST" "rm -f $REMOTE_TGZ"

# Smoke test: POST a known-broken case and look for the expected rule fire.
echo
echo "smoke test:"
echo "  curl https://dw.ramsden-international.com/m-explain --data-binary 'let x = 1; in x'"
