#!/usr/bin/env python3
"""Validate the minimal, high-coverage Quick Brown Fox demo project."""

from __future__ import annotations

import json
import os
import re
import sys
import tomllib
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
VAULT = Path(os.environ.get("FLYNT_DEMO_VAULT", ROOT / "fixtures/demo-vault")).resolve()
BOARD_ID = "564f64c1-2e9c-4c7d-bc6d-84ecf0f5c6c1"

REQUIRED = {
    "README.md",
    "notes/Release Brief.md",
    "notes/Release Runbook.md",
    "diagrams/Architecture.md",
    "diagrams/Architecture.d2",
    "boards/Launch Dashboard.md",
    "boards/Launch Dashboard.board",
    "drawings/System Map.md",
    "drawings/System Map.excalidraw",
    "flows/Release Flow.md",
    "flows/Release Flow.flow",
}
TASK_STATES = {"todo", "in_progress", "done", "archived"}
COLUMNS = {"Ready", "Blocked", "Doing", "Done", "Archive"}
TRANSIENT_PARTS = {".DS_Store", "node_modules", "local", "runtime"}


def fail(message: str) -> None:
    print(f"demo-vault contract: {message}", file=sys.stderr)
    raise SystemExit(1)


def frontmatter(path: Path, artifact: str = "task") -> dict:
    raw = path.read_text()
    if not raw.startswith("+++\n"):
        fail(f"{path.relative_to(VAULT)} lacks TOML {artifact} frontmatter")
    end = raw.find("\n+++", 4)
    if end < 0:
        fail(f"{path.relative_to(VAULT)} has unterminated frontmatter")
    return tomllib.loads(raw[4:end])


def main() -> int:
    files = {p.relative_to(VAULT).as_posix() for p in VAULT.rglob("*") if p.is_file()}
    missing = REQUIRED - files
    if missing:
        fail(f"missing required artifacts: {sorted(missing)}")

    transient = [p for p in VAULT.rglob("*") if p.is_file() and TRANSIENT_PARTS.intersection(p.parts)]
    if transient:
        fail(f"transient state committed: {[str(p.relative_to(VAULT)) for p in transient]}")

    markdown = [p for p in VAULT.rglob("*.md") if "tasks" not in p.parts]
    if len(markdown) != 7:
        fail(f"expected exactly 7 human-facing markdown files, found {len(markdown)}")

    brief = (VAULT / "notes/Release Brief.md").read_text()
    for signal in ("| Constraint | Choice |", "> [!WARNING]", "[[diagrams/Architecture|Architecture]]"):
        if signal not in brief:
            fail(f"release brief missing coverage signal {signal!r}")

    wrappers = {
        "boards/Launch Dashboard.md": "![[Launch Dashboard.board]]",
        "drawings/System Map.md": "![[System Map.excalidraw]]",
        "flows/Release Flow.md": "![[Release Flow.flow]]",
        "diagrams/Architecture.md": "![[Architecture.d2]]",
    }
    for relative, embed in wrappers.items():
        wrapper_body = (VAULT / relative).read_text()
        if embed not in wrapper_body:
            fail(f"{relative} does not embed {embed}")
        if relative.startswith(("diagrams/", "drawings/", "boards/", "flows/")):
            body = re.sub(
                r"\A(?:\+\+\+|---)\n.*?\n(?:\+\+\+|---)\n?",
                "",
                wrapper_body,
                flags=re.S,
            ).strip()
            if body != embed:
                fail(f"{relative} must be a pure artifact wrapper; put prose in a linked note")

    board = json.loads((VAULT / "boards/Launch Dashboard.board").read_text())
    if len(board.get("cells", [])) != 4 or {c["id"] for c in board["cells"]} != {"hero", "stats", "status", "route"}:
        fail("design board must contain the four QBF coverage cells")
    if any("h-full" not in c.get("content", {}).get("html", "") for c in board["cells"]):
        fail("every design-board cell must fill its grid area")

    drawing = json.loads((VAULT / "drawings/System Map.excalidraw").read_text())
    element_types = {element.get("type") for element in drawing.get("elements", [])}
    if not {"rectangle", "text", "arrow"}.issubset(element_types):
        fail("drawing must cover shapes, labels, and connections")

    flow_path = VAULT / "flows/Release Flow.flow"
    flow_raw = flow_path.read_text()
    flow_fm = frontmatter(flow_path, "flow")
    if flow_fm.get("kind") != "flow" or flow_fm.get("data", {}).get("schema_version") != 1:
        fail("flow must use canonical kind=flow TOML frontmatter at schema version 1")
    flow_body = re.sub(r"\A\+\+\+\n.*?\n\+\+\+\n?", "", flow_raw, flags=re.S)
    flow = json.loads(flow_body)
    if len(flow.get("nodes", [])) < 5 or len(flow.get("edges", [])) < 5:
        fail("flow must cover branching and a feedback path")
    socket_types = {socket.get("ty") for node in flow["nodes"] for socket in node.get("sockets", [])}
    if not {"Release", "Issue", "Artifact"}.issubset(socket_types):
        fail("flow must contain typed Release, Issue, and Artifact sockets")

    tasks = sorted((VAULT / "tasks").rglob("*.md"))
    if len(tasks) != 6:
        fail(f"expected exactly 6 tasks, found {len(tasks)}")
    states, columns, priorities, tags = set(), set(), set(), set()
    has_external = has_execution = has_due = False
    for path in tasks:
        data = frontmatter(path).get("data", {})
        if data.get("board") != BOARD_ID:
            fail(f"{path.name} points at the wrong board")
        states.add(data.get("status"))
        columns.add(data.get("column"))
        priorities.add(data.get("priority"))
        tags.update(data.get("tags", []))
        has_external |= bool(data.get("external_refs"))
        has_execution |= bool(data.get("execution"))
        has_due |= bool(data.get("due_date"))
    if states != TASK_STATES or columns != COLUMNS:
        fail(f"task state coverage incomplete: states={states}, columns={columns}")
    if priorities != {0, 1, 2, 3}:
        fail(f"task priority coverage incomplete: {priorities}")
    if not (has_external and has_execution and has_due and {"blocked", "sentry", "archived"}.issubset(tags)):
        fail("task metadata coverage is incomplete")

    scenarios = json.loads((ROOT / "site/screenshots/demo-vault-scenarios.json").read_text())
    write = next((s for s in scenarios["scenarios"] if s["id"] == "write-surface"), None)
    if not write or write.get("doc") != "notes/Release Brief.md":
        fail("write screenshot must target the QBF release brief")

    links = re.findall(r"\[\[([^]|]+)", "\n".join(path.read_text() for path in markdown))
    if len(set(links)) < 6:
        fail("knowledge graph coverage is too sparse")

    print(f"demo-vault QBF contract passed: {len(markdown)} documents, {len(tasks)} tasks, 4 visual artifact types")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
