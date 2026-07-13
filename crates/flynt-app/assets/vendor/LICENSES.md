# Vendored Dependencies

## CodeMirror 6 (`codemirror.bundle.js`)
- License: MIT
- Source: https://codemirror.net
- Packages: @codemirror/view, @codemirror/state, @codemirror/language, @codemirror/commands, @codemirror/search, @codemirror/autocomplete, @codemirror/lang-markdown, @codemirror/language-data, @codemirror/theme-one-dark, @lezer/markdown, @lezer/highlight

## Excalidraw (`excalidraw.bundle.js`, `excalidraw.css`)
- License: MIT
- Source: https://github.com/excalidraw/excalidraw
- Pinned package: `@excalidraw/excalidraw` 0.18.1
- Build recipe: `build/excalidraw/package.json` + `build/excalidraw/package-lock.json`
- Includes React 18.3.1 (MIT) as a bundled dependency
- Rebuild: `npm --prefix crates/flynt-app/build/excalidraw ci && npm --prefix crates/flynt-app/build/excalidraw run build`
- Verification: `python3 scripts/check-editor-bundles.py`

## XYFlow (`flow.bundle.js`)
- License: MIT
- Source: https://github.com/xyflow/xyflow
- Pinned package: `@xyflow/react` 12.11.2
- Build recipe: `build/flow/package.json` + `build/flow/package-lock.json`
- Includes React 18.3.1 (MIT) as a bundled dependency
- Verification: `python3 scripts/check-editor-bundles.py`
