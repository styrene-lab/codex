#!/usr/bin/env python3
"""Audit Flynt open-idle idempotency for a vault.

The harness snapshots project files before and after a manual Flynt session and
reports semantic file mutations. It intentionally excludes .git internals and
can optionally exclude known local runtime paths when the goal is content-sync
noise analysis rather than local-state analysis.

Usage:
  scripts/flynt-open-idle-audit.py /path/to/vault -- cargo run -p flynt-app

The script launches the command with FLYNT_VAULT=<vault>, waits for Enter, then
terminates the process and compares before/after snapshots.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import signal
import subprocess
import sys
import time
from dataclasses import asdict, dataclass
from pathlib import Path
from typing import Iterable

DEFAULT_IGNORES = (
    ".git/",
    ".DS_Store",
)

LOCAL_STATE_IGNORES = (
    ".flynt/local/",
    ".flynt/runtime/",
    ".omegon/",
    "ai/",
)


@dataclass(frozen=True)
class FileRecord:
    sha256: str
    size: int


@dataclass
class AuditReport:
    vault: str
    command: list[str]
    elapsed_seconds: float
    ignores: list[str]
    added: list[str]
    removed: list[str]
    modified: list[str]
    git_status_before: list[str] | None
    git_status_after: list[str] | None


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(description="Audit Flynt open-idle filesystem idempotency")
    parser.add_argument("vault", type=Path, help="Project/vault root to audit")
    parser.add_argument(
        "--include-local-state",
        action="store_true",
        help="Do not ignore .flynt/local/.flynt/runtime/.omegon/ai local-runtime paths",
    )
    parser.add_argument(
        "--json-output",
        type=Path,
        help="Optional path to write the structured audit report as JSON",
    )
    parser.add_argument(
        "--fail-on-change",
        action="store_true",
        help="Exit non-zero when added/removed/modified files are detected",
    )
    parser.add_argument(
        "--duration",
        type=float,
        help="Run for this many seconds instead of waiting for Enter (useful for automation)",
    )
    parser.add_argument(
        "command",
        nargs=argparse.REMAINDER,
        help="Command to launch after '--', e.g. -- cargo run -p flynt-app",
    )
    args = parser.parse_args()
    if args.command and args.command[0] == "--":
        args.command = args.command[1:]
    if not args.command:
        parser.error("launch command is required after '--'")
    return args


def should_ignore(rel: str, ignores: Iterable[str]) -> bool:
    return any(rel == pat.rstrip("/") or rel.startswith(pat) for pat in ignores)


def snapshot(root: Path, ignores: Iterable[str]) -> dict[str, FileRecord]:
    records: dict[str, FileRecord] = {}
    for path in sorted(root.rglob("*")):
        if not path.is_file():
            continue
        rel = path.relative_to(root).as_posix()
        if should_ignore(rel, ignores):
            continue
        try:
            data = path.read_bytes()
        except OSError as exc:
            print(f"warning: could not read {rel}: {exc}", file=sys.stderr)
            continue
        records[rel] = FileRecord(hashlib.sha256(data).hexdigest(), len(data))
    return records


def git_status(root: Path) -> list[str] | None:
    if not (root / ".git").exists():
        return None
    result = subprocess.run(
        ["git", "status", "--porcelain=v1"],
        cwd=root,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.PIPE,
        check=False,
    )
    if result.returncode != 0:
        print(f"warning: git status failed: {result.stderr.strip()}", file=sys.stderr)
        return None
    return [line for line in result.stdout.splitlines() if line]


def terminate(process: subprocess.Popen[bytes]) -> None:
    if process.poll() is not None:
        return
    try:
        process.send_signal(signal.SIGTERM)
        process.wait(timeout=8)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait(timeout=8)


def main() -> int:
    args = parse_args()
    vault = args.vault.expanduser().resolve()
    if not vault.is_dir():
        print(f"vault does not exist or is not a directory: {vault}", file=sys.stderr)
        return 2

    ignores = list(DEFAULT_IGNORES)
    if not args.include_local_state:
        ignores.extend(LOCAL_STATE_IGNORES)

    before = snapshot(vault, ignores)
    git_before = git_status(vault)

    env = os.environ.copy()
    env["FLYNT_VAULT"] = str(vault)
    print(f"Launching: {' '.join(args.command)}")
    print(f"FLYNT_VAULT={vault}")
    print("Interact with Flynt now. Press Enter here when the idle audit window is complete.")
    started = time.monotonic()
    process = subprocess.Popen(args.command, env=env)
    try:
        if args.duration is None:
            input()
        else:
            time.sleep(max(args.duration, 0.0))
    finally:
        terminate(process)
    elapsed = time.monotonic() - started

    after = snapshot(vault, ignores)
    git_after = git_status(vault)

    before_keys = set(before)
    after_keys = set(after)
    added = sorted(after_keys - before_keys)
    removed = sorted(before_keys - after_keys)
    modified = sorted(k for k in before_keys & after_keys if before[k] != after[k])

    report = AuditReport(
        vault=str(vault),
        command=args.command,
        elapsed_seconds=round(elapsed, 3),
        ignores=ignores,
        added=added,
        removed=removed,
        modified=modified,
        git_status_before=git_before,
        git_status_after=git_after,
    )

    print("\n=== Flynt open-idle audit ===")
    print(f"Vault: {report.vault}")
    print(f"Elapsed: {report.elapsed_seconds}s")
    print(f"Added: {len(added)}")
    for item in added:
        print(f"  + {item}")
    print(f"Removed: {len(removed)}")
    for item in removed:
        print(f"  - {item}")
    print(f"Modified: {len(modified)}")
    for item in modified:
        print(f"  M {item}")
    if git_before is not None or git_after is not None:
        print("Git status before:")
        for line in git_before or []:
            print(f"  {line}")
        print("Git status after:")
        for line in git_after or []:
            print(f"  {line}")

    if args.json_output:
        args.json_output.parent.mkdir(parents=True, exist_ok=True)
        args.json_output.write_text(json.dumps(asdict(report), indent=2) + "\n")
        print(f"Wrote JSON report: {args.json_output}")

    changed = bool(added or removed or modified)
    if changed and args.fail_on_change:
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
