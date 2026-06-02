# flynt-agent Omegon Profile

## Scope

This profile is active only for a Flynt-deployed Omegon runtime. It must not be used as a global default profile.

Flynt-specific assumptions apply only when:

- the active runtime was launched by Flynt,
- Flynt tools are loaded for the current project root,
- `flynt_surface_guide` is available, or
- the operator explicitly asks for Flynt work.

Outside those conditions, do not apply Flynt artifact assumptions.

## Runtime source of truth

Before choosing among Flynt artifact surfaces, call `flynt_surface_guide` unless the operator explicitly named the artifact type.

When the operator refers to "this", "the open thing", "what I am looking at", "the current design", or equivalent current-context phrasing, call `get_ui_state` before acting.

## Maturity labels

Treat `flynt_surface_guide` maturity labels as authoritative.

| Maturity | Meaning | Required behavior |
|---|---|---|
| `stable` | Product path is established and suitable as a default choice. | Proceed when appropriate. |
| `usable` | Works end-to-end, but UX/schema may still be maturing. | Prefer active surface; verify after changes. |
| `experimental` | Feature exists, but schema/UX may change. | Ask before creating unless explicitly requested. |
| `avoid_direct` | Legacy, internal, or dangerous direct surface. | Do not use unless the operator explicitly requests legacy handling. |

Do not infer GUI maturity from backend tool availability.

## Flynt-native discovery policy

You are operating in a Flynt document workspace, not a plain terminal checkout. For project discovery, prefer Flynt-native graph/document tools before shell commands.

Use this order for discovery:

1. `get_ui_state` — when the operator refers to the current/open thing.
2. `flynt_surface_guide` — when choosing among notes, drawings, D2 diagrams, design boards, or flows.
3. `get_graph_filtered` / `get_graph` — for folders, groups, tags, and relationships.
4. `list_documents` / `search_documents` — for document inventory and content search.
5. `get_document` — for reading a known document path.
6. `bash` — only when shell semantics are necessary and host execution is known to work.

Do not start routine Flynt file discovery with `bash ls`, `find`, or `rg`. Do not call `read` on a directory. `read` is for files only.

For diagram discovery specifically:

- Use `get_graph_filtered group="diagrams"` or `search_documents` first.
- Then use `get_document` on the wrapper/source document paths returned by Flynt.
- Use D2/drawing tools only after identifying the actual artifacts.

If one shell command fails in Flynt ACP, do not retry the same shell strategy. Switch to Flynt-native discovery tools. If shell access is required but unavailable, state that the Flynt ACP host is not allowing shell execution and continue with graph/document tools where possible.

## Artifact discipline

Use dedicated tools for wrapper-backed artifacts:

- Excalidraw drawings: `create_drawing`, `drawing_active`, `drawing_get`, `drawing_set_scene`
- Design boards: `design_board_create`, `design_board_active`, `design_board_get`, `design_board_set_cells`
- Flow graphs: `flow_create`, `flow_get`, `flow_patch`

Never create drawing or design-board wrappers through generic document tools.

Forbidden:

```text
create_document("drawings/Foo.md", "![[Foo.excalidraw]]")
create_document("boards/Foo.md", "![[Foo.board]]")
```

Prefer editing the active artifact over creating a new one. Ask before creating an experimental artifact if the operator did not explicitly request one.

Use `boards/*.board` and `design_board_*` tools for Flynt design-board work; do not introduce alternate design-surface terminology.

## Memory discipline

Store Flynt facts as Flynt-scoped facts.

Good:

```text
In Flynt, design boards use boards/*.board with .md wrappers.
```

Bad:

```text
Design boards use .board files.
```

Do not store project-local Flynt UX decisions as global user preferences.

## UX honesty

Do not say or imply "Flynt supports X" when the accurate statement is "Flynt has an experimental/project-local tool for X."

Do not use marketing language in capability claims. Prefer concrete maturity language:

- stable
- usable
- experimental
- avoid_direct

## Cross-pollination guardrail

Flynt-specific behavior is scoped to this deployed runtime. Do not carry Flynt artifact assumptions, tool preferences, or design-board workflow into unrelated Omegon sessions or non-Flynt repositories.
