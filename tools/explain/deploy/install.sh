#!/usr/bin/env bash
# install.sh — server-side installer for the M explainer.
#
# Runs on vsprod via:  ssh vsprod 'sudo bash -s /tmp/explain-m.tgz' < install.sh
# First arg is the path to the tarball to extract.

set -euo pipefail

TGZ="${1:?usage: install.sh <tarball>}"
LIB=/usr/local/share/explain-m
CGI=/usr/lib/cgi-bin/explain-m.cgi
CONF=/etc/apache2/proxy-conf.d/explain-m.conf

STAGE=$(mktemp -d /tmp/explain-m-stage.XXXXXX)
trap 'rm -rf "$STAGE"' EXIT

tar -xzf "$TGZ" -C "$STAGE"

# Sanity: all expected files present.
for f in lexical.pl unicode_tables.pl error_rules.pl cgi.pl \
         explain-m.cgi explain-m.conf; do
    [ -e "$STAGE/$f" ] || { echo "missing: $f" >&2; exit 1; }
done

install -d -m 0755 "$LIB"
# Remove stale files from previous deploys before installing fresh set.
rm -f "$LIB"/*.pl
install -m 0644 "$STAGE"/lexical.pl        "$LIB/"
install -m 0644 "$STAGE"/unicode_tables.pl "$LIB/"
install -m 0644 "$STAGE"/error_rules.pl    "$LIB/"
install -m 0644 "$STAGE"/cgi.pl            "$LIB/"
install -m 0755 "$STAGE"/explain-m.cgi     "$CGI"
install -m 0644 "$STAGE"/explain-m.conf    "$CONF"

apache2ctl -t
systemctl reload apache2

echo "installed: $CGI"
echo "library:   $LIB"
echo "apache:    $CONF (reloaded)"
