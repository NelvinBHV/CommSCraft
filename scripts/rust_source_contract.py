"""Layout-insensitive structural checks of Rust source (not a Rust parser)."""
from __future__ import annotations


def code_without_layout(source: str) -> str:
    """Remove whitespace and comments; mask string literal contents.

    Ignore literal/comment text rather than treating it as executable guards.
    Actual behavior remains covered by Rust tests and historical hashes.
    """
    out: list[str] = []
    i = 0
    n = len(source)
    while i < n:
        if source.startswith("//", i):
            stop = source.find("\n", i + 2)
            i = n if stop < 0 else stop
            continue
        if source.startswith("/*", i):
            depth = 1
            i += 2
            while i < n and depth:
                if source.startswith("/*", i):
                    depth += 1
                    i += 2
                elif source.startswith("*/", i):
                    depth -= 1
                    i += 2
                else:
                    i += 1
            if depth:
                raise ValueError("unterminated Rust comment")
            continue
        prefix = 2 if source.startswith("br", i) else (1 if source[i] == "r" else 0)
        if prefix:
            j = i + prefix
            while j < n and source[j] == "#":
                j += 1
            if j < n and source[j] == '"':
                end_token = '"' + "#" * (j - i - prefix)
                end = source.find(end_token, j + 1)
                if end < 0:
                    raise ValueError("unterminated Rust raw string")
                out.append('""')
                i = end + len(end_token)
                continue
        if source[i] == '"' or source.startswith('b"', i):
            if source[i] == "b":
                i += 1
            i += 1
            while i < n:
                if source[i] == "\\":
                    i += 2
                elif source[i] == '"':
                    i += 1
                    break
                else:
                    i += 1
            else:
                raise ValueError("unterminated Rust string")
            out.append('""')
            continue
        if source[i].isspace():
            i += 1
            continue
        out.append(source[i])
        i += 1
    return "".join(out)
