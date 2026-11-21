#!/usr/bin/env python3
"""Fails if a GitHub Actions workflow could change the repository (issue #92) or runs an action
that is not pinned to a commit (issue #213).

Workflows run with read-only tokens and never push: everything reaches main through a pull
request. Checked line by line, comments ignored: write permissions, `permissions:
write-all`, git push, the privileged pull_request_target trigger and persisted credentials.
Every `uses:` names an action by its full 40-hex commit SHA (a tag can be moved); local
actions (`./…`) are allowed.
"""
import pathlib
import re
import sys

FORBIDDEN = [
    (re.compile(r"^\s*[a-z-]+:\s*write\b"), "write permission"),
    (re.compile(r"^\s*permissions:\s*write-all\b"), "write-all permission"),
    (re.compile(r"\bgit\s+push\b"), "git push"),
    (re.compile(r"\bpull_request_target\b"), "pull_request_target trigger"),
    (re.compile(r"^\s*persist-credentials:\s*true\b"), "persisted credentials"),
]


USES = re.compile(r"^\s*(?:-\s*)?uses:\s*(\S+)")
PINNED = re.compile(r"^[\w.-]+/[\w./-]+@[0-9a-f]{40}$")


def problems(text):
    for number, line in enumerate(text.splitlines(), 1):
        code = line.split("#", 1)[0]
        for pattern, what in FORBIDDEN:
            if pattern.search(code):
                yield number, what
        used = USES.match(code)
        if used and not used.group(1).startswith("./") and not PINNED.match(used.group(1)):
            yield number, f"action not pinned to a commit SHA: {used.group(1)}"


def main(paths):
    failed = False
    for path in paths:
        for number, what in problems(pathlib.Path(path).read_text(encoding="utf-8")):
            print(f"{path}:{number}: {what}")
            failed = True
    return 1 if failed else 0


if __name__ == "__main__":
    args = sys.argv[1:] or sorted(str(p) for p in pathlib.Path(".github/workflows").glob("*.y*ml"))
    sys.exit(main(args))
