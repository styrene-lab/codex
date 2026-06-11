+++
title = "Massive Scale Regression Harness"
tags = ["design","performance","testing"]
+++

# Massive Scale Regression Harness

---
title: Massive Scale Regression Harness
status: exploring
tags: [design, performance, testing]
---

# Massive Scale Regression Harness

## Problem

Performance regressions are currently discovered manually through visible rainbow spinners. The project needs scale budgets that fail before release.

## Direction

Add a generated huge-vault fixture and budget checks.

Example generator:

```text
scripts/generate-perf-vault.py --docs 100000 --links 1000000
```

Do not commit generated huge fixtures; generate them locally/CI as needed.

## Budgets

Initial targets:

- shell first paint: under 500ms after process launch in cached-state mode
- tab activation visible: under 100ms
- graph status visible: under 100ms
- graph first interactive frame: under 500ms for medium graphs
- watcher setup: non-blocking relative to first paint

## Acceptance criteria

- Static checker catches dangerous UI hot-path calls.
- Synthetic graph/index benchmarks run in CI or release validation.
- Release checklist includes large-workspace launch and graph smoke validation.
