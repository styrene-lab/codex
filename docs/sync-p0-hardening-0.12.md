---
title: Sync P0 Hardening Findings for 0.12.0
date: 2026-06-07
status: exploring
tags: [sync, p0, release, excalidraw, git, icloud]
---

# Sync P0 Hardening Findings for 0.12.0

This document records concrete P0 findings discovered while validating the dedicated `flynt-vault-sync` vault across two Macs.

## Validation baseline

Dedicated private repo:

- GitHub: `cwilson613/flynt-vault-sync`
- Local Mac path: `/Users/wilson/workspace/flynt-vault-sync`
- Remote Mac path: `/Users/wilson/workspace/flynt-vault-sync`
- Remote Mac: `192.168.0.64`, `chriss-MacBook-Pro-2`
- Remote runtime: `omegon 0.26.13`, `cargo 1.95.0`

Remote Flynt opened the vault and rendered the seeded `Sync Documents` lens successfully.

---

# P0-1 — Excalidraw view rewrites drawings on open

## Severity

P0 for sync. Viewing a drawing must not dirty tracked project files.

## Evidence

Opening/rendering `drawings/Sync Sketch.md` on the remote Mac rewrote the sibling `.excalidraw` file without an intentional edit.

Original seed file:

```json
{
  "type": "excalidraw",
  "version": 2,
  "source": "flynt-sync-validation",
  "elements": [],
  "appState": { "viewBackgroundColor": "#06080e" },
  "files": {}
}
```

After opening in Flynt:

```json
{"type":"excalidraw","version":2,"elements":[],"appState":{"viewBackgroundColor":"transparent","gridSize":20}}
```

Observed diff:

```diff
-  "source": "flynt-sync-validation",
-  "appState": { "viewBackgroundColor": "#06080e" },
-  "files": {}
+  "appState":{"viewBackgroundColor":"transparent","gridSize":20}
```

This is semantic loss, not just formatting.

## Likely cause

`crates/flynt-app/src/views/excalidraw.rs` starts an autosave loop with:

```js
let lastSaved = '';
...
if (window._excalidrawLatest && window._excalidrawLatest !== lastSaved) {
    lastSaved = window._excalidrawLatest;
    window._excSaveQueue.push(lastSaved);
}
```

On mount, the browser-side Excalidraw bridge appears to emit normalized scene JSON. Since `lastSaved` starts empty, the first normalized scene is treated as an edit and written back by Rust:

```rust
std::fs::write(&abs, &data)
```

The same save path also auto-exports SVG, producing `drawings/*.svg` from a view-only action.

## Required fix

Autosave must not write on initial mount. It should be armed only after the initial scene has been observed or after explicit user interaction.

Minimum approach:

```js
let lastSaved = null;
let armed = false;

if (!armed) {
  lastSaved = window._excalidrawLatest;
  armed = true;
  continue;
}

if (window._excalidrawLatest !== lastSaved) {
  lastSaved = window._excalidrawLatest;
  queueSave(lastSaved);
}
```

Better approach:

- pass original file content into the JS save bridge as baseline,
- ignore mount-time normalization,
- save only after user input/change after mount.

SVG auto-export must also be gated behind intentional save or explicit export.

## Acceptance test

1. Start with clean Git working tree.
2. Open a drawing wrapper in Flynt.
3. Wait at least 5 seconds.
4. Run `git status`.
5. Expect no changes to:
   - `drawings/*.excalidraw`
   - `drawings/*.svg`
6. Intentionally edit the drawing.
7. Verify `.excalidraw` saves only after edit.

---

# P0-2 — Opening/configuring Flynt dirties broad project state

## Severity

P0 for Git auto-sync. Auto-sync cannot safely stage all files if opening Flynt generates local/runtime artifacts and tracked content rewrites.

## Evidence

After opening/configuring the remote vault, `git status` showed:

```text
 M Notes/Conflict Candidate.md
 M Notes/Remote-Smoke-20260607T102310Z.md
 M Notes/Welcome.md
 M Projects/Alpha/Overview.md
 M README.md
 M Tasks/Sync Checklist.md
 M drawings/Sync Sketch.excalidraw
 M drawings/Sync Sketch.md
?? .flynt/config.toml
?? .flynt/runtime/forge-sync.db
?? .flynt/runtime/omegon.toml
?? .flynt/runtime/operator-settings.json
?? .flynt/local/registry/project-registry.snapshot.json
?? .flynt/templates/
?? .omegon/
?? ai/
?? drawings/Sync Sketch.svg
```

Some modifications are intentional canonicalization; others are local/generated state.

## Current mitigation applied to validation repo

Updated `.gitignore` on the remote validation repo to ignore obvious local/generated artifacts:

```gitignore
.DS_Store
*.tmp

# Flynt machine-local runtime/cache
.flynt/local/
.omegon/
ai/
.flynt/runtime/forge-sync.db
.flynt/runtime/operator-settings.json
.flynt/runtime/omegon.toml

# Generated snapshots/exports; source files remain tracked
.flynt/local/registry/project-registry.snapshot.json
drawings/*.svg
```

Remaining dirty/untracked files after this are narrower:

```text
 M .gitignore
 M Notes/*.md
 M Projects/**/*.md
 M README.md
 M Tasks/*.md
 M drawings/Sync Sketch.excalidraw
 M drawings/Sync Sketch.md
?? .flynt/config.toml
?? .flynt/templates/
```

## Required fix/policy

Flynt needs a clear portability policy:

Commit/portable:

- `.flynt/config.toml`
- `.flynt/lenses/*.toml`
- markdown notes/tasks/project docs
- `.excalidraw` source files when intentionally edited

Ignore/local/generated:

- `.flynt/local/`
- `.omegon/`
- `ai/`
- `.flynt/runtime/forge-sync.db`
- `.flynt/runtime/operator-settings.json`
- `.flynt/runtime/omegon.toml`
- `.flynt/local/registry/project-registry.snapshot.json`
- generated exports such as `drawings/*.svg`

Needs decision:

- `.flynt/templates/` — commit only if user-authored; ignore if generated defaults.

## Acceptance test

1. Create fresh validation repo.
2. Open in Flynt.
3. Configure Git sync.
4. Close without editing project content.
5. `git status` must contain only intentional config changes, not runtime/cache/export artifacts.

---

# P0-3 — Git auto-sync stages too broadly

## Severity

P0 for Git auto-sync.

## Evidence

`GitSync::auto_commit()` uses:

```rust
index.add_all(["*"].iter(), IndexAddOption::DEFAULT, None)?;
```

This respects `.gitignore`, but the ignore policy is currently incomplete and project opening generates many artifacts.

## Risk

Auto-sync can commit:

- local runtime files,
- generated registry snapshots,
- SVG exports,
- conflict markers,
- accidental normalization rewrites.

## Required fix

Before enabling auto-sync broadly:

- complete `.gitignore` policy,
- add tests proving generated/local artifacts are not staged,
- block auto-commit while merge/conflict state exists.

## Acceptance test

Seed local/generated files and a normal note, run `auto_commit()`, assert only the normal note/config intended files are committed.

---

# P0-4 — Git conflict/divergence safety remains unresolved

## Severity

P0 for Git sync beyond simple fast-forward push/pull.

## Evidence

`GitSync::pull()` attempts non-fast-forward merge via libgit2 but does not clearly create a merge commit for non-conflicting merges before `cleanup_state()`.

`start_auto_sync()` currently continues after conflict with backoff, so a later loop can begin with `auto_commit()`.

## Required fix

For 0.12.0, safest policy is fast-forward-only unless full merge commit behavior is implemented and tested.

Conflict must be terminal for auto-sync until explicit operator resolution.

## Acceptance tests

- fast-forward pull updates files and leaves status clean,
- divergent non-conflicting histories are either safely merged with a merge commit or blocked,
- conflicting histories never auto-commit conflict markers,
- auto-sync halts on conflict.

---

# Immediate P0 work order

1. Fix Excalidraw autosave-on-mount rewrite.
2. Finish Git ignore/portability policy and apply it in project creation.
3. Add an auto-commit ignore-protection test.
4. Make auto-sync halt on conflict.
5. Choose fast-forward-only or implement real merge commit for divergent Git pulls.

Do not enable Git auto-sync by default until these are complete.
