#!/usr/bin/env python3
"""Parse MicrosoftDocs/query-docs per-function markdown pages into JSON.

Reads from tools/ms-docs-mirror/query-languages/m/*.md and emits
mrsflow/stdlib-reference/<Namespace>.json — one file per M namespace,
each containing a list of {name, signature, description, examples}.

Run from repo root: `python3 tools/extract_ms_docs.py`
"""
import json
import re
import sys
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
SRC = REPO / "tools" / "ms-docs-mirror" / "query-languages" / "m"
OUT = REPO / "mrsflow" / "stdlib-reference"

FRONTMATTER_RE = re.compile(r"^---\n(.*?)\n---\n", re.DOTALL)
TITLE_RE = re.compile(r'^title:\s*"?(.+?)"?\s*$', re.MULTILINE)
HEADING_RE = re.compile(r"^##\s+(.+?)\s*$", re.MULTILINE)
TAG_RE = re.compile(r"</?(?:pre|b|i|code)>", re.IGNORECASE)
USAGE_RE = re.compile(
    r"\*\*Usage\*\*\s*\n+```(?:powerquery-m)?\s*\n(.*?)\n```",
    re.DOTALL,
)
OUTPUT_RE = re.compile(
    r"\*\*Output\*\*\s*\n+(?:`(.+?)`|```(?:powerquery-m)?\s*\n(.*?)\n```)",
    re.DOTALL,
)


def section(body: str, name: str) -> str | None:
    """Return everything between '## <name>' and the next '## ' heading."""
    matches = list(HEADING_RE.finditer(body))
    for i, m in enumerate(matches):
        if m.group(1).strip().lower() == name.lower():
            start = m.end()
            end = matches[i + 1].start() if i + 1 < len(matches) else len(body)
            return body[start:end].strip()
    return None


def all_example_sections(body: str) -> str:
    """Concatenate every section whose heading matches `Example` or
    `Example <N>` (some pages use `## Example 1`, `## Example 2`, ...)."""
    matches = list(HEADING_RE.finditer(body))
    out: list[str] = []
    pat = re.compile(r"^Examples?(?:\s+\d+)?$", re.IGNORECASE)
    for i, m in enumerate(matches):
        if pat.match(m.group(1).strip()):
            start = m.end()
            end = matches[i + 1].start() if i + 1 < len(matches) else len(body)
            out.append(body[start:end].strip())
    return "\n\n".join(out)


def clean_signature(raw: str) -> str:
    """Strip HTML tags and collapse whitespace into a single line."""
    cleaned = TAG_RE.sub("", raw)
    return " ".join(cleaned.split())


def parse_examples(example_section: str) -> list[dict]:
    """Pull (usage, output) pairs out of an Example section."""
    examples = []
    # Use finditer to walk usage blocks in document order.
    usages = list(USAGE_RE.finditer(example_section))
    outputs = list(OUTPUT_RE.finditer(example_section))
    # Pair usage[i] with the nearest output that follows it.
    for u in usages:
        out_str = None
        for o in outputs:
            if o.start() > u.end():
                out_str = (o.group(1) or o.group(2) or "").strip()
                break
        examples.append({"usage": u.group(1).strip(), "output": out_str or ""})
    return examples


def parse_file(path: Path) -> dict | None:
    text = path.read_text(encoding="utf-8")
    fm = FRONTMATTER_RE.match(text)
    if not fm:
        return None
    title_match = TITLE_RE.search(fm.group(1))
    if not title_match:
        return None
    name = title_match.group(1).strip().strip('"').strip("'")
    if "." not in name:
        return None  # not a Namespace.Function page

    body = text[fm.end():]
    syntax = section(body, "Syntax")
    if not syntax:
        return None  # not a function reference page
    about = section(body, "About") or ""
    example_blob = all_example_sections(body)

    return {
        "name": name,
        "signature": clean_signature(syntax),
        "description": " ".join(about.split()),
        "examples": parse_examples(example_blob),
    }


def main() -> int:
    if not SRC.is_dir():
        print(f"source missing: {SRC}", file=sys.stderr)
        return 1
    OUT.mkdir(parents=True, exist_ok=True)

    by_namespace: dict[str, list[dict]] = {}
    skipped = 0
    total = 0
    for md in sorted(SRC.glob("*.md")):
        total += 1
        info = parse_file(md)
        if not info:
            skipped += 1
            continue
        ns = info["name"].split(".", 1)[0]
        by_namespace.setdefault(ns, []).append(info)

    for ns, fns in sorted(by_namespace.items()):
        fns.sort(key=lambda f: f["name"])
        (OUT / f"{ns}.json").write_text(
            json.dumps(fns, indent=2, ensure_ascii=False) + "\n",
            encoding="utf-8",
        )

    func_count = sum(len(v) for v in by_namespace.values())
    print(
        f"parsed {func_count} functions across {len(by_namespace)} namespaces "
        f"(scanned {total} files, skipped {skipped} non-function pages)"
    )
    for ns in sorted(by_namespace):
        print(f"  {ns:20s} {len(by_namespace[ns]):4d}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
