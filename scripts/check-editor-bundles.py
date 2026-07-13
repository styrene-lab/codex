#!/usr/bin/env python3
"""Verify checked-in editor bundles match clean, locked npm builds."""

from __future__ import annotations

import hashlib
import subprocess
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SURFACES = {
    "excalidraw": ["excalidraw.bundle.js", "excalidraw.css"],
    "flow": ["flow.bundle.js"],
    "editor": ["editor_bridge.bundle.js"],
}
VENDOR = ROOT / "crates/flynt-app/assets/vendor"


def digest(path: Path) -> str:
    return hashlib.sha256(path.read_bytes()).hexdigest()


def main() -> int:
    failures: list[str] = []
    for name, artifacts in SURFACES.items():
        build_dir = ROOT / "crates/flynt-app/build" / name
        before = {artifact: digest(VENDOR / artifact) for artifact in artifacts}
        subprocess.run(["npm", "ci", "--ignore-scripts"], cwd=build_dir, check=True)
        subprocess.run(["npm", "run", "build"], cwd=build_dir, check=True)
        for artifact in artifacts:
            after = digest(VENDOR / artifact)
            if after != before[artifact]:
                failures.append(f"{artifact} is stale or non-reproducible")

    if failures:
        print("editor bundle verification failed:", file=sys.stderr)
        for failure in failures:
            print(f"- {failure}", file=sys.stderr)
        return 1
    print("editor bundles reproduce from pinned lockfiles")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
