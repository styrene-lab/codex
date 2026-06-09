---
title: Release Runbook
tags: [release, runbook, demo]
---

# Release Runbook

A compact release checklist used by the demo vault screenshots.

## 0.12.2 focus

- Consolidate Flynt-generated state under `.flynt/local/` and `.flynt/runtime/`.
- Keep Omegon-owned project agent state under `.omegon/`.
- Refresh the public documentation site.
- Capture demo-vault screenshots for the site.

## Checks

```sh
python3 scripts/check-release-surfaces.py
python3 scripts/check-site-screenshots.py
npm --prefix site run build
```

Related: [[notes/Product Brief|Product Brief]], [[docs/Architecture Notes|Architecture Notes]].
