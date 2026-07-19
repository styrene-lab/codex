#!/usr/bin/env python3
"""Create, verify, and prune isolated Flynt Candidate vault snapshots."""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import sys
from datetime import datetime, timezone

PROVENANCE = ".flynt-candidate-source.json"
MANIFEST = ".flynt-candidate-manifest.json"
EXCLUDED_PARTS = {".git"}


def canonical(path: Path) -> Path:
    return path.expanduser().resolve()


def hash_file(path: Path) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as handle:
        for chunk in iter(lambda: handle.read(1024 * 1024), b""):
            digest.update(chunk)
    return digest.hexdigest()


def inventory(root: Path) -> list[dict[str, object]]:
    records: list[dict[str, object]] = []
    for path in sorted(root.rglob("*")):
        relative = path.relative_to(root)
        if any(part in EXCLUDED_PARTS for part in relative.parts):
            continue
        if relative.as_posix() in {PROVENANCE, MANIFEST}:
            continue
        if path.is_symlink():
            records.append(
                {"path": relative.as_posix(), "kind": "symlink", "target": os.readlink(path)}
            )
        elif path.is_file():
            records.append(
                {
                    "path": relative.as_posix(),
                    "kind": "file",
                    "size": path.stat().st_size,
                    "sha256": hash_file(path),
                }
            )
    return records


def ensure_safe_destination(source: Path, parent: Path) -> None:
    try:
        parent.relative_to(source)
    except ValueError:
        return
    raise ValueError("snapshot destination must not be inside the source vault")


def copy_snapshot(source: Path, destination: Path) -> None:
    symlinks = [
        path.relative_to(source).as_posix()
        for path in source.rglob("*")
        if ".git" not in path.relative_to(source).parts and path.is_symlink()
    ]
    if symlinks:
        preview = ", ".join(symlinks[:8])
        raise ValueError(
            f"source vault contains symlinks that could escape snapshot isolation: {preview}"
        )

    def ignore(_directory: str, names: list[str]) -> set[str]:
        return {".git"}.intersection(names)

    shutil.copytree(source, destination, symlinks=False, ignore=ignore)


def create(source_arg: str, parent_arg: str, retain: int) -> Path:
    source = canonical(Path(source_arg))
    parent = canonical(Path(parent_arg))
    if not source.is_dir():
        raise ValueError(f"source vault does not exist: {source}")
    parent.mkdir(parents=True, exist_ok=True)
    ensure_safe_destination(source, parent)

    created = datetime.now(timezone.utc)
    stamp = created.strftime("%Y%m%dT%H%M%SZ")
    base = parent / f"{source.name}-candidate-{stamp}"
    destination = base
    suffix = 1
    while destination.exists():
        destination = Path(f"{base}-{suffix}")
        suffix += 1

    copy_snapshot(source, destination)
    records = inventory(destination)
    provenance = {
        "schema": "flynt-candidate-snapshot/v1",
        "source": str(source),
        "created_at": created.isoformat().replace("+00:00", "Z"),
        "writable_snapshot": True,
        "source_git_excluded": True,
    }
    manifest = {
        "schema": "flynt-candidate-manifest/v1",
        "algorithm": "sha256",
        "file_count": sum(record["kind"] == "file" for record in records),
        "entries": records,
    }
    (destination / PROVENANCE).write_text(json.dumps(provenance, indent=2) + "\n")
    (destination / MANIFEST).write_text(json.dumps(manifest, indent=2) + "\n")
    verify(destination)
    prune(parent, source.name, retain, keep=destination)
    return destination


def verify(snapshot_arg: Path) -> None:
    snapshot = canonical(snapshot_arg)
    provenance_path = snapshot / PROVENANCE
    manifest_path = snapshot / MANIFEST
    if not provenance_path.is_file() or not manifest_path.is_file():
        raise ValueError(f"not a managed Candidate snapshot: {snapshot}")
    provenance = json.loads(provenance_path.read_text())
    manifest = json.loads(manifest_path.read_text())
    if provenance.get("schema") != "flynt-candidate-snapshot/v1":
        raise ValueError("unsupported Candidate provenance schema")
    if manifest.get("schema") != "flynt-candidate-manifest/v1":
        raise ValueError("unsupported Candidate manifest schema")
    expected = manifest.get("entries")
    actual = inventory(snapshot)
    if expected != actual:
        expected_map = {entry["path"]: entry for entry in expected or []}
        actual_map = {entry["path"]: entry for entry in actual}
        changed = sorted(set(expected_map) | set(actual_map))
        changed = [path for path in changed if expected_map.get(path) != actual_map.get(path)]
        preview = ", ".join(changed[:8])
        raise ValueError(f"Candidate snapshot integrity check failed: {preview}")
    if manifest.get("file_count") != sum(entry["kind"] == "file" for entry in actual):
        raise ValueError("Candidate manifest file count is inconsistent")


def prune(parent_arg: Path, vault_name: str, retain: int, keep: Path | None = None) -> None:
    if retain < 1:
        raise ValueError("retention must be at least 1")
    parent = canonical(parent_arg)
    prefix = f"{vault_name}-candidate-"
    snapshots = sorted(
        (
            path
            for path in parent.iterdir()
            if path.is_dir() and path.name.startswith(prefix) and (path / PROVENANCE).is_file()
        ),
        key=lambda path: path.name,
        reverse=True,
    )
    retained = 0
    for path in snapshots:
        if keep is not None and path == keep:
            retained += 1
            continue
        if retained < retain:
            retained += 1
            continue
        shutil.rmtree(path)


def parser() -> argparse.ArgumentParser:
    result = argparse.ArgumentParser(description=__doc__)
    commands = result.add_subparsers(dest="command", required=True)
    create_cmd = commands.add_parser("create")
    create_cmd.add_argument("source")
    create_cmd.add_argument("parent")
    create_cmd.add_argument("--retain", type=int, default=5)
    verify_cmd = commands.add_parser("verify")
    verify_cmd.add_argument("snapshot")
    prune_cmd = commands.add_parser("prune")
    prune_cmd.add_argument("parent")
    prune_cmd.add_argument("vault_name")
    prune_cmd.add_argument("--retain", type=int, default=5)
    return result


def main() -> int:
    args = parser().parse_args()
    try:
        if args.command == "create":
            print(create(args.source, args.parent, args.retain))
        elif args.command == "verify":
            verify(Path(args.snapshot))
            print(f"Verified Candidate snapshot: {canonical(Path(args.snapshot))}")
        else:
            prune(Path(args.parent), args.vault_name, args.retain)
    except (OSError, ValueError, json.JSONDecodeError) as error:
        print(f"candidate snapshot error: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
