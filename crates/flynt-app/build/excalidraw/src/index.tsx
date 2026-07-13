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
  unmount(): void;
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
    let scene: Record<string, unknown>;
    try {
      const parsed: unknown = JSON.parse(sceneJson);
      if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
        throw new Error("scene root must be an object");
      }
      scene = parsed as Record<string, unknown>;
    } catch (error) {
      this._root.unmount();
      this._root = null;
      throw new Error(`Invalid Excalidraw scene JSON: ${error instanceof Error ? error.message : String(error)}`);
    }
    // Excalidraw invokes onChange once with its restored/normalized initial
    // scene. Consume exactly that callback as the baseline. Time-based arming
    // can either persist a late mount callback or discard a fast user edit.
    let sawInitialChange = false;
    this._root.render(
      <Excalidraw
        initialData={scene}
        excalidrawAPI={(api) => { this._api = api; }}
        onChange={(elements, appState, files) => {
          if (!sawInitialChange) {
            sawInitialChange = true;
            return;
          }
          onChange(serializeAsJSON(elements, appState, files, "local"));
        }}
      />,
    );
  },
  unmount() {
    this._root?.unmount();
    this._root = null;
    this._api = null;
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
