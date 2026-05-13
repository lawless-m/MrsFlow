#!/usr/bin/env python3
"""Report mrsflow stdlib coverage vs MicrosoftDocs reference.

For each namespace in mrsflow/stdlib-reference/<NS>.json, list the
functions we've bound across mrsflow-core/src/eval/stdlib/*.rs vs the
ones Microsoft documents. Per-namespace breakdown plus a totals line.

Run from repo root: `python3 tools/stdlib_coverage.py`
Optional: `python3 tools/stdlib_coverage.py Text Number` to filter.
"""
import json
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
STDLIB_DIR = REPO / "mrsflow-core" / "src" / "eval" / "stdlib"
REF_DIR = REPO / "mrsflow" / "stdlib-reference"

# Match a stdlib binding tuple: ("Namespace.Name", ...
BINDING_RE = re.compile(r'\(\s*"([A-Z][A-Za-z0-9]*\.[A-Za-z0-9]+)"')


def bound_names() -> set[str]:
    names: set[str] = set()
    for rs in sorted(STDLIB_DIR.glob("*.rs")):
        names.update(BINDING_RE.findall(rs.read_text(encoding="utf-8")))
    return names


def ms_names_by_ns() -> dict[str, set[str]]:
    out: dict[str, set[str]] = {}
    for jf in sorted(REF_DIR.glob("*.json")):
        ns = jf.stem
        names = {f["name"] for f in json.loads(jf.read_text(encoding="utf-8"))}
        out[ns] = names
    return out


def main() -> int:
    if not REF_DIR.is_dir():
        print(f"missing reference dir: {REF_DIR} — run tools/extract_ms_docs.py first",
              file=sys.stderr)
        return 1
    bound = bound_names()
    by_ns = ms_names_by_ns()

    filter_ns = set(sys.argv[1:]) if len(sys.argv) > 1 else None

    total_ms = 0
    total_have = 0
    print(f"{'namespace':20s}  have / total  missing")
    print("-" * 70)
    for ns in sorted(by_ns):
        if filter_ns and ns not in filter_ns:
            continue
        ms = by_ns[ns]
        have = ms & bound
        missing = sorted(ms - bound)
        total_ms += len(ms)
        total_have += len(have)
        missing_preview = ", ".join(missing[:4])
        if len(missing) > 4:
            missing_preview += f", +{len(missing) - 4} more"
        print(f"{ns:20s}  {len(have):3d} / {len(ms):3d}    {missing_preview}")

    print("-" * 70)
    print(f"{'TOTAL':20s}  {total_have:3d} / {total_ms:3d}")

    # Anything bound that MS doesn't document (e.g. #table, #date, #datetime,
    # #duration — synthetic intrinsics) — surface as a sanity check.
    all_ms = set().union(*by_ns.values())
    extras = sorted(b for b in bound if b not in all_ms and "." in b)
    if extras and not filter_ns:
        print()
        print(f"bound but not in MS reference ({len(extras)}):")
        for e in extras:
            print(f"  {e}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
