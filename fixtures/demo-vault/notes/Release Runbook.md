---
title: Release Runbook
tags: [runbook, release]
---

# Release Runbook

Use this short path after reading the [[notes/Release Brief|Release Brief]].

1. Open [[diagrams/Architecture|Architecture]] and confirm the file-first boundary.
2. Move **Review visual contrast** out of Blocked after approval.
3. Inspect [[drawings/System Map|System Map]], [[flows/Release Flow|Release Flow]], and [[boards/Launch Dashboard|Launch Dashboard]].
4. Run:

   ```sh
   python3 scripts/check-demo-vault.py
   python3 scripts/check-site-screenshots.py
   ```

5. Mark **Ship the QBF release** done.

- [x] Scope fixed
- [ ] Visual review approved
- [ ] Release shipped

> [!TIP]
> A failed check returns work to Doing; it never silently publishes.
