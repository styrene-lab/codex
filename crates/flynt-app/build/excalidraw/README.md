# Excalidraw webview bundle

Pinned Excalidraw adapter that owns the `window.FlyntExcalidraw` API consumed by
Flynt's desktop webview. The generated JavaScript and CSS are committed under
`crates/flynt-app/assets/vendor/` so running Flynt does not require Node.js.

## Build

```sh
npm ci
npm run build
```

The lockfile pins Excalidraw, React, TypeScript, esbuild, and all transitive
packages. Do not use `npm audit fix --force`: Excalidraw 0.18.1 is the latest
published package and its remaining advisories are in its upstream
Mermaid/Chevrotain graph. Upgrade Excalidraw as a tested dependency train.

From the repository root, verify every checked-in editor artifact with:

```sh
python3 scripts/check-editor-bundles.py
```

The verifier performs clean locked installs, rebuilds each artifact, compares
its digest with the committed output, and removes generated `node_modules/`
directories before exiting.

## Public API

```js
window.FlyntExcalidraw.mount(elementId, sceneJson, onChange);
window.FlyntExcalidraw.unmount();
await window.FlyntExcalidraw.exportSvg();
```

Invalid scene JSON is rejected rather than converted to an empty drawing. The
first Excalidraw `onChange` emission is consumed as the restored-scene baseline;
subsequent emissions are forwarded as user edits.
