---
title: Quick Brown Fox
aliases: [QBF, Demo Project]
tags: [demo, release]
---

# Quick Brown Fox

Ship one tiny release that exercises every Flynt surface. Start with the [[notes/Release Brief|Release Brief]], follow the [[diagrams/Architecture|Architecture]], then use [[notes/Release Runbook|Release Runbook]].

> [!NOTE]
> The quick brown fox jumps over the lazy dog: one coherent project, complete surface coverage, no filler.

| Surface | Artifact | Signal |
| --- | --- | --- |
| Write + Graph | [[notes/Release Brief|Release Brief]] | links, metadata, rich Markdown |
| Tasks | Launch board | ready, doing, blocked, done, archived |
| Design | [[boards/Launch Dashboard|Launch Dashboard]] | board, drawing, D2, flow |
| Omegon | `Ship the QBF release` | executable task contract |

- [x] Define the release
- [ ] Clear the blocked review
- [ ] Publish after validation

```sh
python3 scripts/check-demo-vault.py
```

External reference: [Flynt](https://github.com/styrene-lab/flynt).
