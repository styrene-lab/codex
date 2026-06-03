// Flynt editor bridge compatibility adapter.
//
// This bundle owns the public `window.FlyntEditor` API while the legacy
// notes.rs CM6 setup still creates `window._flyntCM`. The next extraction
// slice should move EditorView creation into this bundle as well.

interface LegacyEditorView {
  state: {
    doc: { length: number; toString(): string; line(lineNumber: number): { from: number } };
    selection: {
      main: { anchor: number; head: number };
      ranges: Array<{ anchor: number; head: number }>;
    };
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

function attachView(view: LegacyEditorView, initialContent = ""): FlyntEditorApi {
  window._flyntCM = view;
  return install(initialContent);
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

    replaceSelection: (text) => {
      const cm = currentView();
      if (!cm) return { ok: false, reason: "not-mounted" };
      cm.dispatch(cm.state.replaceSelection(String(text ?? "")));
      cm.focus();
      return { ok: true };
    },

    saveNow: () => {
      const cm = currentView();
      if (!cm) return { ok: false, reason: "not-mounted" };
      window._flyntNotify?.("save", cm.state.doc.toString());
      return { ok: true };
    },

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

window.FlyntEditorCompat = { install, attachView };

export {};
