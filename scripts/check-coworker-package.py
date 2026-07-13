#!/usr/bin/env python3
"""Validate the macOS coworker demo ZIP before distribution."""

from __future__ import annotations

import plistlib
import re
import subprocess
import sys
import tempfile
import zipfile
from pathlib import Path

MAX_ARCHIVE_BYTES = 30 * 1024 * 1024
MAX_UNCOMPRESSED_BYTES = 90 * 1024 * 1024
MAX_EXECUTABLE_BYTES = 55 * 1024 * 1024
HASHED_ASSET_RE = re.compile(r"^(?P<stem>.+)-dxh[0-9a-f]+(?P<suffix>\.[^.]+)$")


def fail(message: str) -> None:
    raise SystemExit(f"coworker package invalid: {message}")


def main() -> None:
    if len(sys.argv) != 2:
        fail("usage: check-coworker-package.py <zip>")
    archive = Path(sys.argv[1]).resolve()
    if not archive.is_file():
        fail(f"archive does not exist: {archive}")
    if archive.stat().st_size > MAX_ARCHIVE_BYTES:
        fail(
            f"archive is {archive.stat().st_size / 1024 / 1024:.1f} MiB; "
            f"limit is {MAX_ARCHIVE_BYTES / 1024 / 1024:.0f} MiB"
        )

    with zipfile.ZipFile(archive) as zipped:
        names = zipped.namelist()
        total_uncompressed = sum(info.file_size for info in zipped.infolist())
        if total_uncompressed > MAX_UNCOMPRESSED_BYTES:
            fail(
                f"archive expands to {total_uncompressed / 1024 / 1024:.1f} MiB; "
                f"limit is {MAX_UNCOMPRESSED_BYTES / 1024 / 1024:.0f} MiB"
            )
        roots = {
            name.split("/", 1)[0]
            for name in names
            if name and name.split("/", 1)[0] != "__MACOSX"
        }
        if len(roots) != 1:
            fail(f"expected one top-level folder, found {sorted(roots)}")
        root = next(iter(roots))
        forbidden = [
            name
            for name in names
            if name.startswith("__MACOSX/")
            or "/._" in name
            or name.endswith("/.DS_Store")
            or "/.flynt/local/" in name
            or "/.git/" in name
        ]
        if forbidden:
            fail(f"contains transient or AppleDouble files: {forbidden[:5]}")
        required = [
            f"{root}/Flynt.app/Contents/MacOS/flynt",
            f"{root}/Flynt.app/Contents/Info.plist",
            f"{root}/Open Flynt Demo.command",
            f"{root}/README.txt",
            f"{root}/Quick Brown Fox/.flynt/config.toml",
            f"{root}/Quick Brown Fox/flows/Release Flow.flow",
        ]
        missing = [name for name in required if name not in names]
        if missing:
            fail(f"missing required entries: {missing}")

        executable = zipped.getinfo(f"{root}/Flynt.app/Contents/MacOS/flynt")
        if executable.file_size > MAX_EXECUTABLE_BYTES:
            fail(
                f"app executable is {executable.file_size / 1024 / 1024:.1f} MiB; "
                "expected a stripped release executable"
            )

        asset_prefix = f"{root}/Flynt.app/Contents/Resources/assets/"
        generated_assets: dict[str, list[str]] = {}
        for name in names:
            if not name.startswith(asset_prefix) or name.endswith("/"):
                continue
            relative = name[len(asset_prefix) :]
            if "/" in relative:
                continue
            match = HASHED_ASSET_RE.match(relative)
            if match:
                logical_name = match.group("stem") + match.group("suffix")
                generated_assets.setdefault(logical_name, []).append(relative)
        duplicates = {
            logical: entries
            for logical, entries in generated_assets.items()
            if len(entries) > 1
        }
        if duplicates:
            sample = next(iter(duplicates.items()))
            fail(f"duplicate generated assets for {sample[0]}: {sample[1]}")

    with tempfile.TemporaryDirectory() as directory:
        destination = Path(directory)
        subprocess.run(["ditto", "-x", "-k", str(archive), str(destination)], check=True)
        package = destination / root
        app = package / "Flynt.app"
        launcher = package / "Open Flynt Demo.command"
        with (app / "Contents/Info.plist").open("rb") as handle:
            info = plistlib.load(handle)
        if info.get("CFBundleIdentifier") != "io.styrene.flynt":
            fail(f"unexpected bundle identifier: {info.get('CFBundleIdentifier')}")
        if not launcher.stat().st_mode & 0o111:
            fail("launcher is not executable")
        subprocess.run(
            ["codesign", "--verify", "--deep", "--strict", "--verbose=2", str(app)],
            check=True,
        )
        subprocess.run(
            [sys.executable, str(Path(__file__).with_name("check-demo-vault.py"))],
            check=True,
            env={"FLYNT_DEMO_VAULT": str(package / "Quick Brown Fox")},
        )

    print(f"coworker package contract passed: {archive.name}")


if __name__ == "__main__":
    main()
