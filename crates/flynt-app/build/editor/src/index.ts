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
  baseExtensions(modules: EditorCompatModules, localExtensions?: unknown[]): unknown[];
  mountEditor(modules: EditorCompatModules, container: HTMLElement, content: string, cursorPos?: number, localExtensions?: unknown[], theme?: unknown): FlyntEditorApi;
  contextMenuExtension(EditorView: { domEventHandlers(handlers: Record<string, unknown>): unknown }): unknown;
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

interface EditorCompatModules {
  EditorState?: { create(config: { doc: string; selection?: { anchor: number }; extensions: unknown[] }): unknown };
  EditorView: { lineWrapping: unknown; updateListener: { of(callback: (update: { docChanged?: boolean }) => void): unknown }; decorations?: { compute(deps: string[], fn: (state: { doc: { lines: number; line(i: number): { from: number; text: string } } }) => unknown): unknown } };
  syntaxHighlighting(style: unknown, config?: unknown): unknown;
  flyntHighlight: unknown;
  defaultHighlightStyle: unknown;
  oneDark: unknown;
  highlightActiveLine(): unknown;
  highlightSpecialChars(): unknown;
  highlightSelectionMatches(): unknown;
  drawSelection(): unknown;
  bracketMatching(): unknown;
  closeBrackets(): unknown;
  searchKeymap: unknown;
  defaultKeymap: unknown;
  history(): unknown;
  historyKeymap: unknown;
  indentWithTab: unknown;
  markdown(config: unknown): unknown;
  markdownLanguage: unknown;
  GFM: unknown;
  languages: unknown;
  createFrontmatterHider?: () => unknown;
  Decoration?: { replace(spec?: { widget?: unknown }): { range(from: number, to: number): unknown }; set(values: unknown[]): unknown };
  WidgetType?: { new(): { eq?(other: unknown): boolean; toDOM?(): HTMLElement } };
  keymap: { of(bindings: unknown[]): unknown };
}


function taskListExtension(modules: EditorCompatModules): unknown | null {
  if (!modules.EditorView.decorations || !modules.Decoration || !modules.WidgetType) return null;
  const Decoration = modules.Decoration;
  const BaseWidget = modules.WidgetType;
  class TaskCheckWidget extends BaseWidget {
    private checked: boolean;
    private lineFrom: number;
    constructor(checked: boolean, lineFrom: number) {
      super();
      this.checked = checked;
      this.lineFrom = lineFrom;
    }
    eq(other: unknown): boolean {
      return other instanceof TaskCheckWidget && other.checked === this.checked;
    }
    toDOM(): HTMLElement {
      const checkbox = document.createElement("input");
      checkbox.type = "checkbox";
      checkbox.checked = this.checked;
      checkbox.className = "cm-task-checkbox";
      checkbox.onclick = (event) => {
        event.preventDefault();
        const view = currentView();
        if (!view) return;
        const line = view.state.doc.lineAt(this.lineFrom);
        const next = this.checked
          ? line.text.replace("[x]", "[ ]").replace("[X]", "[ ]")
          : line.text.replace("[ ]", "[x]");
        view.dispatch({ changes: { from: line.from, to: line.from + line.text.length, insert: next } });
      };
      return checkbox;
    }
  }

  return modules.EditorView.decorations.compute(["doc"], (state) => {
    const decorations: unknown[] = [];
    for (let i = 1; i <= state.doc.lines; i += 1) {
      const line = state.doc.line(i);
      const match = line.text.match(/^(\s*[-*]\s*)\[([ xX])\]\s/);
      if (!match) continue;
      const prefixLength = match[1]?.length ?? 0;
      const checked = match[2] !== " ";
      decorations.push(Decoration.replace({ widget: new TaskCheckWidget(checked, line.from) }).range(line.from + prefixLength, line.from + prefixLength + 3));
      if (prefixLength > 0) decorations.push(Decoration.replace({}).range(line.from, line.from + prefixLength));
    }
    return Decoration.set(decorations);
  });
}

function baseExtensions(modules: EditorCompatModules, localExtensions: unknown[] = []): unknown[] {
  const flyntKeymaps = keymapRegistry(modules.keymap);
  const taskList = taskListExtension(modules);
  return [
    modules.syntaxHighlighting(modules.flyntHighlight),
    modules.oneDark,
    modules.syntaxHighlighting(modules.defaultHighlightStyle, { fallback: true }),
    modules.markdown({
      base: modules.markdownLanguage,
      codeLanguages: modules.languages,
      extensions: modules.GFM,
    }),
    modules.history(),
    modules.drawSelection(),
    modules.highlightActiveLine(),
    modules.highlightSpecialChars(),
    modules.highlightSelectionMatches(),
    modules.bracketMatching(),
    modules.closeBrackets(),
    modules.keymap.of([
      modules.indentWithTab,
      ...(modules.defaultKeymap as unknown[]),
      ...(modules.historyKeymap as unknown[]),
      ...(modules.searchKeymap as unknown[]),
    ]),
    flyntKeymaps.save,
    flyntKeymaps.formatting,
    changeHandlerExtension(modules.EditorView),
    ...(modules.createFrontmatterHider ? [modules.createFrontmatterHider()] : []),
    ...(taskList ? [taskList] : []),
    ...localExtensions,
    modules.EditorView.lineWrapping,
  ];
}



function contextMenuExtension(EditorView: { domEventHandlers(handlers: Record<string, unknown>): unknown }): unknown {
  return EditorView.domEventHandlers({
    contextmenu(event: MouseEvent) {
      event.preventDefault();
      document.getElementById("flynt-ctx-menu")?.remove();
      document.querySelector(".ctx-menu-overlay")?.remove();

      const view = currentView();
      if (!view) return true;
      const sel = view.state.selection.main;
      const hasSelection = sel.anchor !== sel.head;

      const menu = document.createElement("div");
      menu.id = "flynt-ctx-menu";
      menu.className = "ctx-menu";
      menu.style.cssText = `left:${event.clientX}px;top:${event.clientY}px;position:fixed;z-index:1000;`;

      const items = [
        ...(hasSelection ? [
          { id: "bold", label: "Bold", key: "⌘B" },
          { id: "italic", label: "Italic", key: "⌘I" },
          { id: "code", label: "Inline Code", key: "" },
          { id: "strike", label: "Strikethrough", key: "" },
          { id: "link", label: "Link", key: "⌘K" },
          { id: "wikilink", label: "Wikilink", key: "" },
          { id: "sep" },
        ] : []),
        { id: "h1", label: "Heading 1", key: "" },
        { id: "h2", label: "Heading 2", key: "" },
        { id: "h3", label: "Heading 3", key: "" },
        { id: "sep" },
        { id: "bullet", label: "Bullet List", key: "" },
        { id: "task", label: "Task List", key: "" },
        { id: "quote", label: "Blockquote", key: "" },
        { id: "codeblock", label: "Code Block", key: "" },
        { id: "table", label: "Table", key: "" },
        { id: "hr", label: "Horizontal Rule", key: "" },
      ];

      const overlay = document.createElement("div");
      overlay.className = "ctx-menu-overlay";
      overlay.onclick = () => { menu.remove(); overlay.remove(); };

      for (const item of items) {
        if (item.id === "sep") {
          const sep = document.createElement("div");
          sep.className = "ctx-menu-sep";
          menu.appendChild(sep);
          continue;
        }
        const btn = document.createElement("button");
        btn.className = "ctx-menu-item";
        const label = item.label ?? "";
        btn.innerHTML = item.key ? `<span>${label}</span><span class="ctx-menu-key">${item.key}</span>` : label;
        btn.onclick = () => {
          menu.remove();
          overlay.remove();
          dispatchEditorCommand(item.id);
        };
        menu.appendChild(btn);
      }

      document.body.appendChild(overlay);
      document.body.appendChild(menu);
      requestAnimationFrame(() => {
        const r = menu.getBoundingClientRect();
        if (r.right > window.innerWidth) menu.style.left = Math.max(8, window.innerWidth - r.width - 8) + "px";
        if (r.bottom > window.innerHeight) menu.style.top = Math.max(8, window.innerHeight - r.height - 8) + "px";
      });
      return true;
    },
  });
}

function mountEditor(
  modules: EditorCompatModules,
  container: HTMLElement,
  content: string,
  cursorPos = content.length,
  localExtensions: unknown[] = [],
  theme?: unknown,
): FlyntEditorApi {
  if (!modules.EditorState) {
    throw new Error("FlyntEditorCompat.mountEditor requires EditorState");
  }
  const extensions = [
    ...(theme ? [theme] : []),
    ...baseExtensions(modules, localExtensions),
  ];
  const state = modules.EditorState.create({
    doc: content,
    selection: { anchor: cursorPos },
    extensions,
  });
  const view = new (modules.EditorView as unknown as { new(config: { state: unknown; parent: HTMLElement }): LegacyEditorView })({ state, parent: container });
  return attachView(view, content);
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

window.FlyntEditorCompat = { install, attachView, changeHandlerExtension, keymapRegistry, baseExtensions, mountEditor, contextMenuExtension, commandRegistry, dispatchEditorCommand };

export {};
