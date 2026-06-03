// Flynt editor bridge compatibility adapter.
//
// This bundle owns the public `window.FlyntEditor` API while the legacy
// notes.rs CM6 setup still creates `window._flyntCM`. The next extraction
// slice should move EditorView creation into this bundle as well.

interface LegacyEditorView {
  state: {
    doc: { length: number; toString(): string; line(lineNumber: number): { from: number }; lineAt(pos: number): { from: number; to: number; text: string } };
    selection: {
      main: { anchor: number; head: number; from: number; to: number };
      ranges: Array<{ anchor: number; head: number }>;
    };
    sliceDoc(from: number, to: number): string;
    replaceSelection(text: string): unknown;
  };
  scrollDOM: { scrollTop: number; scrollLeft: number };
  focus(): void;
  destroy(): void;
  dispatch(spec: unknown): void;
}

interface SerializedSelection {
  anchor: number;
  head: number;
  ranges?: Array<{ anchor: number; head: number }>;
}

interface SerializedEditorState {
  selection?: SerializedSelection;
  scrollTop: number;
  scrollLeft: number;
  dirty: boolean;
}

interface EditorDocumentSnapshot {
  content: string;
  dirty: boolean;
}

interface BridgeResult {
  ok: boolean;
  reason?: string;
  compatibility?: boolean;
  revision?: number;
}

interface FlyntEditorApi {
  mount(): BridgeResult;
  unmount(): void;
  setDocument(doc: { content?: string } | null, options?: { preserveSelection?: boolean; preserveScroll?: boolean; force?: boolean }): BridgeResult;
  getDocument(): EditorDocumentSnapshot;
  focus(): void;
  getSelection(): SerializedSelection | null;
  replaceSelection(text: string): BridgeResult;
  executeCommand(id: string, payload?: { text?: string }): BridgeResult;
  saveNow(): BridgeResult;
  markSaved(revision?: number, content?: string): BridgeResult;
  isDirty(): boolean;
  getEditorState(): SerializedEditorState;
  restoreEditorState(state: Partial<SerializedEditorState> | null): BridgeResult;
  revealLine(lineNumber: number): BridgeResult;
  reconfigure(): BridgeResult;
}

interface FlyntEditorCompatApi {
  install(initialContent?: string): FlyntEditorApi;
  attachView(view: LegacyEditorView, initialContent?: string): FlyntEditorApi;
  changeHandlerExtension(EditorView: { updateListener: { of(callback: (update: { docChanged?: boolean }) => void): unknown } }): unknown;
  keymapRegistry(keymap: { of(bindings: Array<{ key: string; run(view: LegacyEditorView): boolean }>): unknown }): { save: unknown; formatting: unknown; all: unknown[] };
  commandRegistry(): EditorCommandRegistry;
  dispatchEditorCommand(id: string, payload?: { text?: string }): BridgeResult;
}

declare global {
  interface Window {
    _flyntCM?: LegacyEditorView | null;
    _flyntEditorSavedContent?: string;
    _flyntEditorDirty?: boolean;
    _flyntNotify?: (type: string, data: string) => void;
    CM?: { EditorView?: { scrollIntoView(pos: number, options: unknown): unknown } };
    FlyntEditor?: FlyntEditorApi;
    FlyntEditorCompat?: FlyntEditorCompatApi;
  }
}

function currentView(): LegacyEditorView | null {
  return window._flyntCM ?? null;
}

function keymapRegistry(keymap: { of(bindings: Array<{ key: string; run(view: LegacyEditorView): boolean }>): unknown }): { save: unknown; formatting: unknown; all: unknown[] } {
  const save = keymap.of([
    { key: "Mod-s", run: () => dispatchEditorCommand("save").ok },
    { key: "Mod-e", run: () => dispatchEditorCommand("source-mode").ok },
  ]);

  const formatting = keymap.of([
    { key: "Mod-b", run: () => dispatchEditorCommand("bold").ok },
    { key: "Mod-i", run: () => dispatchEditorCommand("italic").ok },
    { key: "Mod-k", run: () => dispatchEditorCommand("link").ok },
  ]);

  return { save, formatting, all: [save, formatting] };
}

function changeHandlerExtension(EditorView: { updateListener: { of(callback: (update: { docChanged?: boolean }) => void): unknown } }): unknown {
  let saveTimer: ReturnType<typeof setTimeout> | null = null;
  let editTimer: ReturnType<typeof setTimeout> | null = null;
  return EditorView.updateListener.of((update) => {
    if (!update.docChanged) return;
    window._flyntEditorDirty = true;
    if (saveTimer) clearTimeout(saveTimer);
    if (editTimer) clearTimeout(editTimer);

    // Defer stringification so large paste operations don't block the
    // synchronous CM6 update path.
    editTimer = setTimeout(() => {
      const cm = currentView();
      if (cm) window._flyntNotify?.("edit", cm.state.doc.toString());
    }, 300);

    saveTimer = setTimeout(() => {
      const cm = currentView();
      if (cm) window._flyntNotify?.("autosave", cm.state.doc.toString());
    }, 1500);
  });
}

function attachView(view: LegacyEditorView, initialContent = ""): FlyntEditorApi {
  window._flyntCM = view;
  return install(initialContent);
}


interface EditorCommandRegistry {
  execute(id: string, payload?: { text?: string }): BridgeResult;
}

function activeText(view: LegacyEditorView): { from: number; to: number; text: string; line: { from: number; to: number; text: string } } {
  const sel = view.state.selection.main;
  const from = Math.min(sel.anchor, sel.head);
  const to = Math.max(sel.anchor, sel.head);
  return {
    from,
    to,
    text: view.state.sliceDoc(from, to),
    line: view.state.doc.lineAt(sel.head),
  };
}

function wrapSelection(view: LegacyEditorView, before: string, after: string): void {
  const sel = activeText(view);
  const insert = sel.text.startsWith(before) && sel.text.endsWith(after)
    ? sel.text.slice(before.length, -after.length)
    : before + sel.text + after;
  view.dispatch({ changes: { from: sel.from, to: sel.to, insert } });
}

function insertAtLineStart(view: LegacyEditorView, prefix: string): void {
  const { line } = activeText(view);
  const text = line.text;
  if (text.startsWith(prefix)) {
    view.dispatch({ changes: { from: line.from, to: line.from + prefix.length, insert: "" } });
    return;
  }
  const heading = text.match(/^#{1,6}\s/);
  const remove = heading ? heading[0].length : 0;
  view.dispatch({ changes: { from: line.from, to: line.from + remove, insert: prefix } });
}

function insertBlock(view: LegacyEditorView, text: string): void {
  const { line } = activeText(view);
  const pos = line.to;
  view.dispatch({ changes: { from: pos, insert: "\n" + text + "\n" }, selection: { anchor: pos + 1 + text.length } });
}

function commandRegistry(): EditorCommandRegistry {
  return {
    execute(id, payload = {}) {
      const view = currentView();
      if (!view) return { ok: false, reason: "not-mounted" };
      const sel = activeText(view);
      let mutatesDocument = true;
      switch (id) {
        case "bold": wrapSelection(view, "**", "**"); break;
        case "italic": wrapSelection(view, "*", "*"); break;
        case "code": wrapSelection(view, "`", "`"); break;
        case "strike": wrapSelection(view, "~~", "~~"); break;
        case "link": view.dispatch({ changes: { from: sel.from, to: sel.to, insert: "[" + sel.text + "](url)" } }); break;
        case "wikilink": wrapSelection(view, "[[", "]]"); break;
        case "h1": insertAtLineStart(view, "# "); break;
        case "h2": insertAtLineStart(view, "## "); break;
        case "h3": insertAtLineStart(view, "### "); break;
        case "bullet": insertAtLineStart(view, "- "); break;
        case "task": insertAtLineStart(view, "- [ ] "); break;
        case "quote": insertAtLineStart(view, "> "); break;
        case "codeblock": insertBlock(view, "```\n\n```"); break;
        case "table": insertBlock(view, "| Column 1 | Column 2 | Column 3 |\n| --- | --- | --- |\n|  |  |  |"); break;
        case "hr": insertBlock(view, "---"); break;
        case "insert-text": view.dispatch(view.state.replaceSelection(String(payload.text ?? ""))); break;
        case "save": window._flyntNotify?.("save", view.state.doc.toString()); mutatesDocument = false; break;
        case "source-mode": window._flyntNotify?.("mode", "source"); mutatesDocument = false; break;
        default: return { ok: false, reason: "unknown-command" };
      }
      if (mutatesDocument) window._flyntEditorDirty = true;
      view.focus();
      return { ok: true };
    }
  };
}

function dispatchEditorCommand(id: string, payload?: { text?: string }): BridgeResult {
  const registry = commandRegistry();
  return registry.execute(id, payload);
}

function install(initialContent = ""): FlyntEditorApi {
  window._flyntEditorSavedContent = initialContent;
  window._flyntEditorDirty = false;

  const api: FlyntEditorApi = {
    mount: () => ({ ok: true, compatibility: true }),

    unmount: () => {
      const cm = currentView();
      if (cm) {
        try { cm.destroy(); } catch { /* ignore legacy destroy errors */ }
        window._flyntCM = null;
      }
    },

    setDocument: (doc, options = {}) => {
      const cm = currentView();
      if (!cm) return { ok: false, reason: "not-mounted" };
      const content = String(doc?.content ?? "");
      const current = cm.state.doc.toString();
      if (api.isDirty() && !options.force && content !== current) {
        return { ok: false, reason: "unsaved-divergence" };
      }

      const selection = cm.state.selection;
      const scrollTop = cm.scrollDOM.scrollTop;
      const scrollLeft = cm.scrollDOM.scrollLeft;
      cm.dispatch({ changes: { from: 0, to: cm.state.doc.length, insert: content } });
      if (options.preserveSelection) cm.dispatch({ selection });
      if (options.preserveScroll) {
        cm.scrollDOM.scrollTop = scrollTop;
        cm.scrollDOM.scrollLeft = scrollLeft;
      }
      window._flyntEditorSavedContent = content;
      window._flyntEditorDirty = false;
      return { ok: true };
    },

    getDocument: () => {
      const cm = currentView();
      const content = cm ? cm.state.doc.toString() : "";
      return { content, dirty: api.isDirty() };
    },

    focus: () => { currentView()?.focus(); },

    getSelection: () => {
      const cm = currentView();
      if (!cm) return null;
      const main = cm.state.selection.main;
      return {
        anchor: main.anchor,
        head: main.head,
        ranges: cm.state.selection.ranges.map((r) => ({ anchor: r.anchor, head: r.head })),
      };
    },

    replaceSelection: (text) => dispatchEditorCommand("insert-text", { text }),

    executeCommand: (id, payload) => dispatchEditorCommand(id, payload),

    saveNow: () => dispatchEditorCommand("save"),

    markSaved: (revision, content) => {
      const cm = currentView();
      window._flyntEditorSavedContent = content !== undefined ? String(content) : (cm ? cm.state.doc.toString() : "");
      window._flyntEditorDirty = false;
      return { ok: true, revision };
    },

    isDirty: () => {
      const cm = currentView();
      if (!cm) return false;
      return Boolean(window._flyntEditorDirty) || cm.state.doc.toString() !== (window._flyntEditorSavedContent ?? "");
    },

    getEditorState: () => {
      const cm = currentView();
      if (!cm) return { scrollTop: 0, scrollLeft: 0, dirty: false };
      const main = cm.state.selection.main;
      return {
        selection: { anchor: main.anchor, head: main.head },
        scrollTop: cm.scrollDOM.scrollTop,
        scrollLeft: cm.scrollDOM.scrollLeft,
        dirty: api.isDirty(),
      };
    },

    restoreEditorState: (state) => {
      const cm = currentView();
      if (!cm || !state) return { ok: false, reason: "not-mounted" };
      if (state.selection) cm.dispatch({ selection: { anchor: state.selection.anchor, head: state.selection.head } });
      if (typeof state.scrollTop === "number") cm.scrollDOM.scrollTop = state.scrollTop;
      if (typeof state.scrollLeft === "number") cm.scrollDOM.scrollLeft = state.scrollLeft;
      return { ok: true };
    },

    revealLine: (lineNumber) => {
      const cm = currentView();
      if (!cm) return { ok: false, reason: "not-mounted" };
      const line = cm.state.doc.line(Math.max(1, Number(lineNumber) || 1));
      cm.dispatch({
        selection: { anchor: line.from },
        effects: window.CM?.EditorView?.scrollIntoView(line.from, { y: "start", yMargin: 24 }),
      });
      cm.focus();
      return { ok: true };
    },

    reconfigure: () => ({ ok: false, reason: "compatibility-wrapper" }),
  };

  window.FlyntEditor = api;
  return api;
}

window.FlyntEditorCompat = { install, attachView, changeHandlerExtension, keymapRegistry, commandRegistry, dispatchEditorCommand };

export {};
