#!/usr/bin/env python3
"""Check Flynt release-visible surfaces stay in sync.

This guard is intentionally lightweight: it catches forgotten version bumps,
missing changelog sections, stale site release-card copy, and stale site lockfile
metadata before a release commit lands.
"""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def read(path: str) -> str:
    return (ROOT / path).read_text(encoding="utf-8")


def fail(message: str) -> None:
    print(f"release-surface check failed: {message}", file=sys.stderr)
    sys.exit(1)


def cargo_workspace_version() -> str:
    text = read("Cargo.toml")
    match = re.search(r"\[workspace\.package\]\s+version\s*=\s*\"([^\"]+)\"", text)
    if not match:
        fail("could not find [workspace.package] version in Cargo.toml")
    return match.group(1)


def toml_version(path: str) -> str:
    match = re.search(r'^version\s*=\s*"([^"]+)"', read(path), re.MULTILINE)
    if not match:
        fail(f"could not find version in {path}")
    return match.group(1)


def package_json_version(path: str) -> str:
    return json.loads(read(path))["version"]


def main() -> int:
    version = cargo_workspace_version()
    tag = f"v{version}"

    agent_version = toml_version("crates/flynt-agent/manifest.toml")
    if agent_version != version:
        fail(f"crates/flynt-agent/manifest.toml is {agent_version}, expected {version}")

    site_version = package_json_version("site/package.json")
    if site_version != version:
        fail(f"site/package.json is {site_version}, expected {version}")

    lock = json.loads(read("site/package-lock.json"))
    if lock.get("name") != "flynt-site":
        fail("site/package-lock.json root name is not flynt-site")
    if lock.get("version") != version:
        fail(f"site/package-lock.json root version is {lock.get('version')}, expected {version}")
    root_package = lock.get("packages", {}).get("", {})
    if root_package.get("name") != "flynt-site" or root_package.get("version") != version:
        fail("site/package-lock.json packages[''] does not match flynt-site release version")

    changelog = read("CHANGELOG.md")
    if f"## {version}" not in changelog:
        fail(f"CHANGELOG.md is missing a ## {version} section")

    site_index = read("site/src/pages/index.astro")
    if tag not in site_index:
        fail(f"site/src/pages/index.astro does not mention {tag}")

    release_page = read("site/src/pages/docs/release-0-12.astro")
    if version not in release_page:
        fail(f"site/src/pages/docs/release-0-12.astro does not mention {version}")

    print(f"release surfaces are in sync for {tag}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
