#!/usr/bin/env python3
"""Validate site screenshot references and generated screenshot contract."""

from __future__ import annotations

import json
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SITE = ROOT / "site"


def fail(message: str) -> None:
    print(f"site screenshot check failed: {message}", file=sys.stderr)
    sys.exit(1)


def load_json(path: Path):
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except Exception as exc:
        fail(f"could not parse {path.relative_to(ROOT)}: {exc}")


def assert_asset(path: Path) -> None:
    if not path.exists():
        fail(f"missing screenshot asset: {path.relative_to(ROOT)}")
    if path.stat().st_size <= 0:
        fail(f"empty screenshot asset: {path.relative_to(ROOT)}")


def main() -> int:
    data_path = SITE / "src" / "data" / "screenshots.json"
    scenario_path = SITE / "screenshots" / "demo-vault-scenarios.json"
    screenshots = load_json(data_path)
    scenarios = load_json(scenario_path).get("scenarios", [])

    scenario_ids = {scenario.get("id") for scenario in scenarios}
    if not screenshots:
        fail("site/src/data/screenshots.json has no screenshots")

    for item in screenshots:
        sid = item.get("id")
        if sid not in scenario_ids:
            fail(f"screenshot {sid!r} has no matching demo-vault scenario")
        image = item.get("image", "")
        if not image.startswith("/screenshots/"):
            fail(f"screenshot {sid!r} image must live under /screenshots/")
        assert_asset(SITE / "public" / image.removeprefix("/"))
        if not item.get("alt"):
            fail(f"screenshot {sid!r} is missing alt text")
        if not item.get("caption"):
            fail(f"screenshot {sid!r} is missing caption")

    for scenario in scenarios:
        placeholder = scenario.get("placeholder")
        if placeholder:
            assert_asset(SITE / "public" / "screenshots" / placeholder)

    missing_refs = []
    for path in list((SITE / "src").rglob("*.astro")) + list((SITE / "src").rglob("*.json")):
        text = path.read_text(encoding="utf-8")
        for ref in re.findall(r"/screenshots/[^\"'\s)]+", text):
            asset = SITE / "public" / ref.removeprefix("/")
            if not asset.exists():
                missing_refs.append(f"{path.relative_to(ROOT)} -> {ref}")
    if missing_refs:
        fail("missing referenced screenshot assets:\n" + "\n".join(missing_refs))

    print(f"site screenshots are valid: {len(screenshots)} screenshot contract entries")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
