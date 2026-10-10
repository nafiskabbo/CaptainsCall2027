#!/usr/bin/env python3
"""Convert raw <button className="btn ..."> text </button> into the UI kit's
<Button>, so the bevel texture and button faces are declared exactly once.

Only the mechanical cases are handled: a single class list whose every class is
a known variant/modifier, and a text-only child. Anything else (nested spans,
template-literal class names, dynamic children) is left alone for a human.

Run from the repo root:  python3 tools/migrate-buttons.py
"""
from __future__ import annotations

import re
import sys
from pathlib import Path

SRC = Path("src")

# className -> the props the <Button> component needs.
VARIANTS = {
    "btn": [],
    "btn btn--primary": [],
    "btn btn--gray": ['variant="gray"'],
    "btn btn--danger": ['variant="danger"'],
    "btn btn--ghost": ['variant="ghost"'],
    "btn btn--sm": ['size="sm"'],
    "btn btn--sm btn--ghost": ['variant="ghost"', 'size="sm"'],
    "btn btn--ghost btn--sm": ['variant="ghost"', 'size="sm"'],
    "btn btn--ghost btn--icon": ['variant="ghost"', 'className="btn--icon"'],
    "btn btn--icon btn--ghost": ['variant="ghost"', 'className="btn--icon"'],
    "btn btn--primary btn--sm": ['size="sm"'],
    "btn btn--continue": ['variant="continue"'],
}

# class names that mean "handled elsewhere" — never rewrite these.
SKIP = {"btn--icononly"}

# <button className="X" ...attrs...> text </button>  (attributes, then text child)
PATTERNS = [
    # single-line child
    re.compile(
        r'<button\s+className="(btn[^"]*)"([^>]*?)>\s*([^<>]+?)\s*</button>'
    ),
    # child spanning several lines, still text only (no markup inside)
    re.compile(
        r'<button\s+className="(btn[^"]*)"([^>]*?)>\s*\n\s*([^<>\n][^<>]*?)\s*\n\s*</button>'
    ),
    re.compile(
        r"<button\s+className=\{`(btn[^`]*)`\}([^>]*?)>\s*([^<>]+?)\s*</button>"
    ),
]


def split_attrs(attrs: str) -> list[str]:
    """Keep every attribute except the className we replaced.

    Splits on whitespace but respects `{...}` and `"..."` so that object
    literals like `style={{ marginTop: "0.5rem" }}` survive intact.
    """
    out = []
    depth = 0
    buf = []
    in_str = False
    for ch in attrs:
        if in_str:
            buf.append(ch)
            if ch == '"':
                in_str = False
            continue
        if ch == '"':
            in_str = True
            buf.append(ch)
            continue
        if ch == "{":
            depth += 1
        elif ch == "}":
            depth -= 1
        if depth == 0 and ch.isspace():
            if buf:
                out.append("".join(buf))
                buf = []
            continue
        buf.append(ch)
    if buf:
        out.append("".join(buf))
    return out


def convert(text: str) -> tuple[str, int]:
    count = 0
    for pattern in PATTERNS:
        def repl(m: re.Match) -> str:
            nonlocal count
            classes = m.group(1).strip()
            attrs = m.group(2)
            child = m.group(3).strip()
            if classes not in VARIANTS or any(s in classes for s in SKIP):
                return m.group(0)
            kept = split_attrs(attrs)
            props = list(VARIANTS[classes])
            # title/aria-*/disabled/onClick stay on the component; drop empty
            # style={} left behind by the original markup.
            kept = [a for a in kept if not re.match(r'style=\{\}', a)]
            all_props = props + kept
            joined = " ".join(all_props) if all_props else ""
            prefix = f"<Button {joined}".rstrip()
            count += 1
            return f"{prefix}>{child}</Button>"

        text = pattern.sub(repl, text)
    return text, count


def main() -> int:
    if not SRC.exists():
        print("run from the repo root", file=sys.stderr)
        return 2
    total = 0
    for path in sorted(SRC.rglob("*.tsx")):
        original = path.read_text()
        updated, n = convert(original)
        if n:
            path.write_text(updated)
            print(f"  {path}: {n} button(s)")
            total += n
    print(f"\n{total} buttons migrated")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
