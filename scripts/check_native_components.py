#!/usr/bin/env python3
"""Checks the native components the Flatpak bundles from source against their newest
upstream release (issue #256): MIT Kerberos (krb5) is pinned in the manifest by URL and
checksum; a newer release on kerberos.org fails the check, so security fixes are not missed.
The runtime (org.freedesktop.Platform) and the FFmpeg extension come from Flathub and are
updated there; their exact builds are listed per release in components.txt (flatpak workflow).

Usage: check_native_components.py [MANIFEST]
Exit 0: up to date; 1: a newer release exists; 2: the manifest or upstream could not be read.
"""
import pathlib
import re
import sys
import urllib.request

MANIFEST = "packaging/flatpak/io.github.NelvinBHV.CommSCraft.yml"
KRB5_DIST = "https://kerberos.org/dist/krb5/"
# Upstream listings are small HTML pages; anything larger is refused.
MAX_PAGE = 1 << 20


def fetch(url):
    req = urllib.request.Request(url, headers={"User-Agent": "native-components-check"})
    with urllib.request.urlopen(req, timeout=30) as r:
        data = r.read(MAX_PAGE + 1)
    if len(data) > MAX_PAGE:
        raise ValueError(f"{url}: page too large")
    return data.decode("utf-8", "replace")


def version(text):
    return tuple(int(p) for p in text.split("."))


def pinned_krb5(manifest):
    m = re.search(r"kerberos\.org/dist/krb5/[0-9.]+/krb5-([0-9]+(?:\.[0-9]+){1,2})\.tar\.gz", manifest)
    return m.group(1) if m else None


def newest_krb5():
    series = re.findall(r'href="([0-9]+\.[0-9]+)/"', fetch(KRB5_DIST))
    if not series:
        raise ValueError("no krb5 release series found")
    newest = max(series, key=version)
    files = re.findall(r'krb5-([0-9]+(?:\.[0-9]+){1,2})\.tar\.gz"', fetch(f"{KRB5_DIST}{newest}/"))
    if not files:
        raise ValueError(f"no krb5 {newest} tarball found")
    return max(files, key=version)


def main():
    path = pathlib.Path(sys.argv[1] if len(sys.argv) > 1 else MANIFEST)
    try:
        manifest = path.read_text(encoding="utf-8")
    except OSError as e:
        print(f"cannot read {path}: {e}")
        return 2
    pinned = pinned_krb5(manifest)
    if pinned is None:
        print("no pinned krb5 source in the manifest")
        return 2
    try:
        newest = newest_krb5()
    except (OSError, ValueError) as e:
        print(f"cannot read the krb5 releases: {e}")
        return 2
    runtime = re.search(r"runtime-version:\s*'?([0-9.]+)", manifest)
    print(f"runtime org.freedesktop.Platform {runtime.group(1) if runtime else '?'} (Flathub)")
    print(f"krb5 pinned {pinned}, newest upstream {newest}")
    if version(newest) > version(pinned):
        print(f"krb5 {newest} is newer: update the manifest (URL, sha256, signature check)")
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
