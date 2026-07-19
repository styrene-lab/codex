#!/usr/bin/env python3
"""Contract tests for side-by-side Stable/Candidate/Dev launch tooling."""
from pathlib import Path

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
assert "candidate-smoke.sh" in launch_candidate
assert "pkill" not in launch_candidate

assert "candidate-snapshot.py" in prepare_candidate
assert "FLYNT_CANDIDATE_RETAIN" in prepare_candidate

print("daily-driver isolation contracts passed")
