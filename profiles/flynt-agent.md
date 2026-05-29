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

Legacy `canvas`, `.canvas`, and `canvases/` terminology is not the current Flynt design-board surface. Use `boards/*.board` and `design_board_*` tools for new Flynt design-board work.

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
