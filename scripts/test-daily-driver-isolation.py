#!/usr/bin/env python3
"""Contract tests for side-by-side Stable/Candidate/Dev launch tooling."""
from pathlib import Path
import tempfile
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
launch_dev = (ROOT / "scripts/launch-local-app.sh").read_text()
launch_candidate = (ROOT / "scripts/launch-candidate.sh").read_text()
prepare_candidate = (ROOT / "scripts/prepare-candidate.sh").read_text()

assert "io.styrene.flynt.dev" in launch_dev
assert "FLYNT_BUILD_IDENTITY=dev" in launch_dev
assert "pkill" not in launch_dev, "Dev launcher must never use process-name-wide termination"
assert "fixtures/demo-vault" in launch_dev
assert "FLYNT_LAUNCHER_PROFILE" in launch_dev

assert "io.styrene.flynt.candidate" in launch_candidate
assert "FLYNT_BUILD_IDENTITY=candidate" in launch_candidate
assert "prepare-candidate.sh" in launch_candidate
assert "pkill" not in launch_candidate

assert "--exclude='.git/'" in prepare_candidate
assert ".flynt-candidate-source.json" in prepare_candidate

with tempfile.TemporaryDirectory() as tmp:
    tmp_path = Path(tmp)
    source = tmp_path / "vault"
    snapshots = tmp_path / "snapshots"
    source.mkdir()
    (source / "note.md").write_text("canonical\n")
    (source / ".hidden").write_text("state\n")
    (source / ".git").mkdir()
    (source / ".git/config").write_text("remote\n")
    result = subprocess.run(
        [str(ROOT / "scripts/prepare-candidate.sh"), str(source), str(snapshots)],
        capture_output=True,
        text=True,
    )
    assert result.returncode == 0, result.stderr
    candidate = Path(result.stdout.strip())
    assert (candidate / "note.md").read_text() == "canonical\n"
    assert (candidate / ".hidden").read_text() == "state\n"
    assert not (candidate / ".git").exists()
    (candidate / "note.md").write_text("candidate\n")
    assert (source / "note.md").read_text() == "canonical\n"

print("daily-driver isolation contracts passed")
