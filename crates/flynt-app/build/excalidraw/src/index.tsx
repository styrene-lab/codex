import React from "react";
import { createRoot, type Root } from "react-dom/client";
import {
  Excalidraw,
  exportToSvg,
  serializeAsJSON,
} from "@excalidraw/excalidraw";

type ExcalidrawApi = Parameters<NonNullable<React.ComponentProps<typeof Excalidraw>["excalidrawAPI"]>>[0];

interface FlyntExcalidrawBridge {
  _root: Root | null;
  _api: ExcalidrawApi | null;
  mount(containerId: string, sceneJson: string, onChange: (data: string) => void): void;
  exportSvg(): Promise<string>;
}

const bridge: FlyntExcalidrawBridge = {
  _root: null,
  _api: null,
  mount(containerId, sceneJson, onChange) {
    const container = document.getElementById(containerId);
    if (!container) throw new Error(`Excalidraw container not found: ${containerId}`);
    this._root?.unmount();
    this._api = null;
    this._root = createRoot(container);
    let scene: Record<string, unknown> = {};
    try { scene = JSON.parse(sceneJson) as Record<string, unknown>; } catch { /* empty scene */ }
    let mounted = false;
    this._root.render(
      <Excalidraw
        initialData={scene}
        excalidrawAPI={(api) => {
          this._api = api;
          // Excalidraw emits normalized scene data while mounting. Arm change
          // propagation after React has committed that initial state so merely
          // opening a drawing can never become a persisted edit.
          queueMicrotask(() => { mounted = true; });
        }}
        onChange={(elements, appState, files) => {
          if (mounted) onChange(serializeAsJSON(elements, appState, files, "local"));
        }}
      />,
    );
  },
  async exportSvg() {
    if (!this._api) return "";
    const svg = await exportToSvg({
      elements: this._api.getSceneElements(),
      appState: this._api.getAppState(),
      files: this._api.getFiles(),
    });
    return new XMLSerializer().serializeToString(svg);
  },
};

declare global { interface Window { FlyntExcalidraw: FlyntExcalidrawBridge; } }
window.FlyntExcalidraw = bridge;
