# Dual-Lane Navigation Implementation Handoff

## Status

The design phase and first core model slice for dual-lane navigation are complete. The next work should begin UI implementation in the Flynt sidebar.

Primary design node:

```text
docs/dual-lane-project-navigation.md
```

Latest projection-model commit:

```text
1271321 feat(sidebar): add dual-lane projection model
```

## Product model

Flynt navigation should have two complementary lanes:

```text
1. Notes / Text Files
   A real filesystem-style tree for ordinary plaintext files.

2. Artifacts
   A semantic ProjectRegistry-derived tree for multi-file domain artifacts.
```

This preserves Obsidian/Zed-style low-level filesystem agency for notes while preventing artifact wrapper/source/sidecar files from polluting the normal note tree.

## Decisions already made

### Notes/Text Files lane

The Notes/Text Files lane preserves low-level plaintext file agency:

- New File
- New Folder
- rename/move/delete ordinary text files
- real folder hierarchy
- raw source access
- ordinary filesystem mental model

Allowed safe plaintext types:

```text
.md
.markdown
.txt
.toml
.yaml
.yml
.json
.csv
```

Markdown remains the primary note type, but other plaintext files should be allowed as raw text. Richer editors/viewers for common formats can be added later.

The Notes/Text Files lane hides by default:

- artifact wrappers, e.g. `boards/Demo.md`, `drawings/Foo.md`
- artifact sources, e.g. `*.board`, `*.excalidraw`, `*.d2`, `*.flow`
- generated sidecars, e.g. `*.png`, `*.html`, rendered SVGs
- runtime/internal dirs, e.g. `.flynt`, `.flynt/local`, `.omegon`, `.git`, `target`, `node_modules`

### Artifacts lane

The Artifacts lane is semantic and registry-derived. It should be tree-like, but it is not filesystem truth.

Initial groups:

```text
Artifacts
  Boards
  Drawings
  Diagrams
  Flows
```

Each visible item is a domain object, not an implementation file.

Example:

```text
Artifacts
  Boards
    Demo
```

represents backing files such as:

```text
boards/Demo.board
boards/Demo.md
boards/Demo.png
boards/Demo.html
```

but those files are not shown as ordinary note entries.

### Reveal actions bridge semantic artifacts to filesystem truth

There is no global raw-files tree by default.

Power users access backing files from artifact context menus:

- Reveal in Filesystem Explorer
- Reveal Source
- Reveal Wrapper
- Reveal Generated Outputs

This preserves low-level agency without turning implementation files into first-class normal navigation entries.

### Creation affordances

Notes/Text Files:

```text
New File
New Folder
```

These should be separate, standard file-browser actions, not conflated into one ambiguous button.

Artifacts:

```text
Boards    +
Drawings  +
Diagrams  +
Flows     +
```

Each artifact-type `+` invokes that type's semantic creation command and owns creation of source/wrapper/sidecar files.

### Tasks and specs

Tasks and OpenSpec changes are their own semantic domains. They should not be folded into Artifacts.

Future sidebar shape may include:

```text
Tasks
Specs
```

but do not include those in the first Artifacts implementation slice unless explicitly scoped.

### Wrapper source editing

Artifact wrappers are protected from normal note editing/autosave.

Power users may access wrapper source only through explicit context actions such as `Reveal Wrapper Source`.

Committed protection fix:

```text
e28e1cf fix(visual-artifacts): protect wrapper documents from note saves
```

### `.board` is the native design surface

Flynt current design-board surface is:

```text
boards/*.board
design_board_*
```

Do not reintroduce `.canvas`, `canvases/`, `canvas_*`, or `legacy_canvas` as current Flynt surface terminology.

Obsidian Canvas is not in scope unless a future explicit importer is designed. Current product direction is: Flynt has boards, not canvases.

## Relevant completed work

### Project Registry MVP

Project Registry can now discover/project:

- documents
- visual artifacts
- raw assets
- external refs
- evidence streams
- tasks/boards
- OpenSpec changes
- graph edges
- diagnostics

It can persist a generated snapshot:

```text
.flynt/local/registry/project-registry.snapshot.json
```

Important: the persisted snapshot is diagnostic/generated state only. It is **not UI truth**.

Sidebar should use live project/store discovery or a live in-memory `ProjectRegistry`, not the persisted snapshot.

Release-gate commit:

```text
4505681 test(project-registry): add MVP release gate
```

### Board surface fixes

Recent board/rendering fixes:

```text
453a422 fix(design-board): key board view by source path
fbe2794 fix(design-board): remount board surface per source
91faaa2 fix(design-board): reload board from active path
```

These addressed stale board content when switching between board tabs.

## New core sidebar projection model

Added:

```text
crates/flynt-core/src/sidebar_projection.rs
```

Exported from:

```rust
pub mod sidebar_projection;
```

Main type:

```rust
pub struct SidebarProjection {
    pub text_files: Vec<TextFileNavItem>,
    pub artifacts: ArtifactNavGroups,
    pub diagnostics: Vec<SidebarProjectionDiagnostic>,
}
```

Text item:

```rust
pub struct TextFileNavItem {
    pub id: Option<DocumentId>,
    pub path: PathBuf,
    pub title: String,
    pub kind: TextFileKind,
}
```

Text kinds:

```rust
pub enum TextFileKind {
    Markdown,
    PlainText,
    Toml,
    Yaml,
    Json,
    Csv,
}
```

Artifact groups:

```rust
pub struct ArtifactNavGroups {
    pub boards: Vec<ArtifactNavItem>,
    pub drawings: Vec<ArtifactNavItem>,
    pub diagrams: Vec<ArtifactNavItem>,
    pub flows: Vec<ArtifactNavItem>,
}
```

Artifact item:

```rust
pub struct ArtifactNavItem {
    pub id: VisualArtifactId,
    pub title: String,
    pub kind: ArtifactNavKind,
    pub source_path: PathBuf,
    pub wrapper_path: Option<PathBuf>,
    pub render_paths: Vec<RenderArtifact>,
}
```

`ArtifactNavItem::reveal_paths()` returns source, wrapper, and render output paths.

Validation:

```bash
cargo test -p flynt-core sidebar_projection --lib
cargo check -p flynt-core
```

Both passed when the model was committed.

## Demo project state

Demo/test project:

```text
/Users/wilson/Documents/Flynt
```

Current board artifacts:

```text
boards/Demo.board
boards/Demo.md
boards/qrypt-system-architecture.board
boards/qrypt-system-architecture.md
```

`Demo.md` wrapper was repaired to:

```markdown
![[Demo.board]]
```

Interconnected low-level notes were added under:

```text
/Users/wilson/Documents/Flynt/notes/
```

Files:

```text
system-overview.md
api-gateway.md
control-plane.md
worker-pool.md
data-store.md
reliability-risks.md
```

These are intended to populate backlinks and graph connectivity.

## Next implementation slice

Recommended commit scope:

```text
feat(sidebar): render dual-lane notes and artifact navigation
```

### Scope

1. Build a live `ProjectRegistry` from the current app project/store.
2. Convert it to `SidebarProjection`.
3. Render Notes/Text Files section from `projection.text_files`.
4. Render Artifacts section from `projection.artifacts`.
5. Hide artifact wrappers/sources from the notes section.
6. Add initial artifact context actions:
   - Open
   - Reveal Source
   - Reveal Wrapper
   - Reveal Generated Outputs
7. Keep notes low-level creation semantics:
   - New File
   - New Folder

### Avoid in first UI slice

Do not include yet unless explicitly requested:

- task/spec sidebar sections
- drag/drop move
- artifact rename/delete/duplicate
- snapshot-driven UI
- global raw files tree
- `.canvas` terminology/support

## Implementation guidance

Use `SidebarProjection::from_registry(...)` as the projection boundary. Keep UI code from directly rediscovering artifact backing files where possible.

The persisted Project Registry snapshot must not be used as authoritative UI input.

Artifact wrappers should remain protected from normal note saves. Do not route artifact wrappers through normal markdown editing by default.

## Suggested validation commands

```bash
cargo test -p flynt-core sidebar_projection --lib
cargo test -p flynt-app design_board --lib
cargo test -p flynt-app bootstrap --lib
cargo check -p flynt-app
```

If touching the agent surface guide:

```bash
cargo test -p flynt-agent surface_guide --lib
```

## Active/recent test app session

Most recent relaunched test session used:

```bash
FLYNT_PROJECT=/Users/wilson/Documents/Flynt cargo run -p flynt-app --bin flynt
```

If still running, stop it before rebuilding/relaunching.
