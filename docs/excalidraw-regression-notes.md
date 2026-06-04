---
title: Excalidraw Regression Notes
status: blocked
tags: [visual-artifacts, excalidraw, regression]
---

# Excalidraw Regression Notes

## Current state

During the `release/0.12.0` visual-artifact testing pass, Excalidraw integration regressed and should be treated as blocked until the integration seam is reassessed from first principles.

Observed by operator:

1. Initially, switching between two different `.excalidraw` drawings showed the content from whichever drawing was opened first.
2. The two source files are **not identical**:
   - `drawings/Drawing 20260515-095234933.excalidraw`
   - `drawings/v1-1-proposal.excalidraw`
3. After attempted remount/keying changes, Excalidraw drawings render blank/no content.
4. Reverting the mount target back to the actual DOM id did not restore rendering in operator testing.

## Important facts learned

- The rendered Excalidraw mount container id in `ExcalidrawView` is still:

  ```text
  flynt-excalidraw
  ```

- Attempting to mount into a path-derived container id was wrong because that DOM node was not actually rendered.
- The vendored JS exposes `window.FlyntExcalidraw.mount(containerId, sceneJson, onChange)` and internally maintains singleton state:

  ```js
  window.FlyntExcalidraw = {
    _root: null,
    _api: null,
    _onChange: null,
    mount(...),
    getScene(...),
    exportSvg(...),
    unmount(...)
  }
  ```

- The upstream/vendored Excalidraw bundle contains `customData` support and appears to be `@excalidraw/excalidraw` `0.18.0`.

## Failed/insufficient attempts

These commits/changes did **not** resolve the operator-visible issue:

- Keying `ExcalidrawView` at the `NotesView` callsite.
- Clearing stale `doc_data` on tab change.
- Clearing singleton `_root` / `_api` before remount.
- Patching the vendor mount to `replaceChildren()` and clear `_api`.
- Attempting path-derived mount container ids.
- Switching the JS lookup back to `flynt-excalidraw` after the path-derived id caused blank rendering.

## Assessment

The current model is likely wrong: treating Excalidraw as a simple remountable singleton is not reliable.

The next pass should **not** continue incremental blind patches. It should inspect and redesign the seam:

1. Map `ExcalidrawView` DOM structure and lifecycle.
2. Map the vendored wrapper contract and whether it supports repeated mount/unmount safely.
3. Determine whether the right operation on tab/path change is:
   - full unmount/remount,
   - a single persistent mount plus `_api.updateScene(...)`, or
   - one React root per drawing/editor instance.
4. Add minimal browser-console/log instrumentation before changing behavior again.
5. Build a two-file repro test fixture and verify switching behavior manually.

## Recommended next plan

1. Stop changing production behavior until instrumentation identifies which layer is stale:
   - Rust `path/content` values,
   - DOM container identity,
   - `window.FlyntExcalidraw._api`,
   - React root lifecycle,
   - Excalidraw internal scene state.
2. Add temporary debug logging around:
   - `ExcalidrawView(path)` render,
   - loaded content length/hash,
   - JS mount key,
   - container id existence,
   - `getSceneElements().length` after mount.
3. Prefer a new wrapper function such as:

   ```js
   window.FlyntExcalidraw.loadScene(sceneJson)
   ```

   implemented via `_api.updateScene(...)` if the singleton is meant to persist.
4. Only after the seam is understood, remove temporary logging and commit a focused fix.

## Release impact

Do not ship `release/0.12.0` with Excalidraw blank rendering. Either:

- fix the integration seam properly, or
- revert the Excalidraw-specific remount/keying changes and explicitly defer Excalidraw Phase 5 UI behavior.
