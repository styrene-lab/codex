#!/usr/bin/env python3
"""Tests for Candidate snapshot integrity, isolation, and retention."""

from __future__ import annotations

import json
from pathlib import Path
import subprocess
import tempfile

ROOT = Path(__file__).resolve().parents[1]
TOOL = ROOT / "scripts/candidate-snapshot.py"


def run(*args: str, expect: int = 0) -> subprocess.CompletedProcess[str]:
    result = subprocess.run(
        ["python3", str(TOOL), *args], capture_output=True, text=True
    )
    assert result.returncode == expect, result.stderr
    return result


with tempfile.TemporaryDirectory() as tmp:
    root = Path(tmp)
    source = root / "vault"
    parent = root / "snapshots"
    (source / ".flynt").mkdir(parents=True)
    (source / ".flynt/config.toml").write_text('project_name = "Daily"\n')
    (source / "note.md").write_text("canonical\n")
    (source / ".hidden").write_text("state\n")
    (source / ".git").mkdir()
    (source / ".git/config").write_text("remote\n")

    first = Path(run("create", str(source), str(parent), "--retain", "2").stdout.strip())
    run("verify", str(first))
    assert (first / "note.md").read_text() == "canonical\n"
    assert (first / ".hidden").read_text() == "state\n"
    assert not (first / ".git").exists()
    provenance = json.loads((first / ".flynt-candidate-source.json").read_text())
    assert provenance["source"] == str(source.resolve())
    manifest = json.loads((first / ".flynt-candidate-manifest.json").read_text())
    assert manifest["algorithm"] == "sha256"
    assert manifest["file_count"] == 3

    (first / "note.md").write_text("candidate mutation\n")
    assert (source / "note.md").read_text() == "canonical\n"
    failed = run("verify", str(first), expect=1)
    assert "note.md" in failed.stderr

    second = Path(run("create", str(source), str(parent), "--retain", "2").stdout.strip())
    third = Path(run("create", str(source), str(parent), "--retain", "2").stdout.strip())
    assert second.is_dir() and third.is_dir()
    snapshots = [path for path in parent.iterdir() if path.is_dir()]
    assert len(snapshots) == 2
    assert not first.exists()

    nested = source / "snapshots"
    blocked = run("create", str(source), str(nested), expect=1)
    assert "inside the source vault" in blocked.stderr

    linked_source = root / "linked-vault"
    linked_source.mkdir()
    (linked_source / "outside.md").symlink_to(source / "note.md")
    blocked_link = run("create", str(linked_source), str(parent), expect=1)
    assert "symlinks" in blocked_link.stderr

print("Candidate snapshot tests passed")
