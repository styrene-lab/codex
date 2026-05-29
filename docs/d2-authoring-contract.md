# D2 Authoring Contract

This contract defines how Flynt/Omegon agents should author D2 diagrams so the generated SVG is readable, patchable, and robust in Flynt's previewer. It is a source-authoring contract, not a viewer workaround.

## Problem this prevents

D2 can emit multiline Markdown labels as SVG `foreignObject` nodes whose calculated height is too small for the content. In practice this produces clipped text, overlapping labels, and unreadable diagrams. Dense cross-container edges also create spaghetti when one diagram tries to show topology, state authority, event flow, and migration deltas at the same time.

## Hard rules

1. **Do not use multiline `|md` bodies inside ordinary graph nodes.**
   - Allowed: top-level title and compact legends.
   - Disallowed: cards whose body contains heading + multiple detail lines.

2. **Represent details as child nodes, not label paragraphs.**

   Bad:

   ```d2
   queue: |md
     **Command Queue**
     durable queue
     monotonic sequence cursors
     atomic with business writes
     retry + dead-letter
   |
   ```

   Good:

   ```d2
   queue: Command Queue {
     durable: durable queue
     cursors: monotonic sequence cursors
     atomic: atomic with business writes
     retry: retry + dead-letter
   }
   ```

3. **Keep edge labels short.**
   - Good: `enqueue`, `notify`, `sync`, `reads`, `writes`.
   - Bad: prose sentences or explanations.
   - If an explanation matters, make it a separate note/callout node.

4. **One D2 diagram answers one primary question.**
   Split when the diagram mixes more than one of:
   - runtime topology
   - state authority / persistence
   - command/event flow
   - migration delta
   - trust boundary
   - operational lifecycle

5. **Minimize cross-container edges.**
   Prefer local edges inside a container, then a small number of container-level edges between major groups.

6. **Avoid panoramic posters.**
   Target readable aspect ratios: roughly 16:9, 4:3, or at most 3:1. If SVG output becomes ~5:1, split the diagram.

7. **Use classes/styles for relationship semantics instead of long labels.**

   ```d2
   classes: {
     durable-edge: {
       style: { stroke: "#1ab878" stroke-width: 3 }
     }
     notify-edge: {
       style: { stroke: "#2ab4c8" stroke-dash: 4 }
     }
   }

   api -> queue: enqueue { class: durable-edge }
   status -> api: notify { class: notify-edge }
   ```

## Recommended structure

### Topology diagram

Use short component labels and few edges.

```d2
direction: right

operator: Operator
worker: Worker Node
active: Active Manager {
  nginx: NGINX :443
  api: Luffy / Control API
  db: SQLCipher + Honker
}
standby: Standby Manager

operator -> active.nginx: HTTPS
worker -> active.nginx: command/status
active.api -> active.db: reads/writes
active.db -> standby: WAL sync
```

### State authority diagram

Use containers and child nodes. No multiline markdown cards.

```d2
direction: down

sqlcipher: SQLCipher + Honker {
  authority: Authority Tables {
    nodes: nodes / roles / capabilities
    intent: command intent + targets
    trust: license + trust metadata
    certs: CA / cert state / RBAC
  }

  queue: Command Queue {
    durable: durable queue
    cursors: monotonic sequence cursors
    atomic: atomic with business writes
    retry: retry + dead-letter
  }

  audit: Audit Event Log {
    append: append-only stream
    offsets: per-consumer offsets
    history: permanent history
  }
}
```

### Flow diagram

Keep each edge label to one verb phrase.

```d2
direction: right

api: Control API
queue: Command Queue
honker: Honker Notify
worker: Worker
status: Command Status

api -> queue: enqueue
queue -> honker: notify
honker -> worker: wake
worker -> status: update
status -> api: stream
```

## Validation checks

After rendering, inspect generated SVG for suspect `foreignObject` sizing:

```bash
grep -o 'foreignObject[^>]*height="[0-9.]*"' diagrams/*.svg
```

A `height="24"` foreignObject containing detailed multiline content is a failure. A top-level title/legend may use a foreignObject if it renders correctly.

## When to split

Split a D2 file when any of these are true:

- Rendered SVG width is more than 3× height.
- More than three edge families cross between major containers.
- A node label needs more than two detail lines.
- You need a legend to explain most edges.
- The diagram cannot be read at 100–150% zoom in Flynt.

## Non-goals

- Do not compensate for bad D2 source by patching the Flynt viewer.
- Do not use Excalidraw as the default escape hatch for ordinary D2 failures.
- Do not post-process SVG geometry unless renderer bugs make all source-level fixes impossible.
