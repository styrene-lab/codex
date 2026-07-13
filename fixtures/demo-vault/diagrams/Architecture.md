---
title: Architecture
tags: [architecture, diagram]
---

# Architecture

Flynt reads portable project files and derives local indexes; optional sync transports the same files without changing their meaning.

![[Architecture.d2]]

| Boundary | Owns |
| --- | --- |
| Project | notes, tasks, boards, drawings, flows |
| Local runtime | indexes, caches, sessions |
| Sync provider | transport, not canonical structure |

```text
project files → index → Write / Graph / Tasks / Design
                    ↘ Omegon tools
```

The editable spatial view is [[drawings/System Map|System Map]]. Execution follows [[flows/Release Flow|Release Flow]]. Return to the [[notes/Release Brief|Release Brief]].
