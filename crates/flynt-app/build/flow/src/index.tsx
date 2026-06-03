// Flynt flow viewer/editor — React + @xyflow/react host.
//
// Phase 3: editable. Operator can drag nodes, draw new edges, and delete
// selected elements; changes flow back to disk via a debounced onChange
// callback registered by the Rust view. Phase 4 will add agent tools.
//
// Public API matches FlyntExcalidraw's shape on purpose so the Rust view
// component can copy that pattern verbatim:
//
//   window.FlyntFlow.mount(elementId, flowJson, {
//     readOnly: false,
//     onChange: (json) => { /* called debounced; json is a Flow body */ },
//   });
//   window.FlyntFlow.unmount();
//
// The bundle is loaded eagerly via document::Script in app.rs.

import * as React from "react";
import { createRoot, Root } from "react-dom/client";
import {
  ReactFlow,
  Background,
  Controls,
  Edge,
  Handle,
  MiniMap,
  Node,
  NodeChange,
  EdgeChange,
  Connection,
  Position,
  ReactFlowProvider,
  applyEdgeChanges,
  applyNodeChanges,
  addEdge,
} from "@xyflow/react";
import reactFlowCss from "@xyflow/react/dist/style.css";

// ── Schema mirror ───────────────────────────────────────────────────────────
//
// These types mirror the `Flow` struct from `flynt-flow` (Rust). They're
// duplicated rather than generated because the Rust → TS code-gen story
// in this repo hasn't landed; with one schema and a small surface,
// hand-mirroring is cheaper than wiring up ts-rs / specta.

interface FlowJson {
  meta?: { title?: string; description?: string };
  nodes: FlowNodeJson[];
  edges: FlowEdgeJson[];
}

interface FlowNodeJson {
  id: string;
  kind: string;
  position: [number, number];
  data?: Record<string, unknown>;
  sockets?: SocketJson[];
}

interface SocketJson {
  name: string;
  direction: "input" | "output";
  ty?: string;
}

interface FlowEdgeJson {
  id: string;
  source: { node: string; socket: string };
  target: { node: string; socket: string };
}

interface FlowNodeDefinition {
  kind: string;
  label: string;
  description: string;
  category: "Core";
  sockets: SocketJson[];
  defaultData?: Record<string, unknown>;
}

const CORE_NODE_DEFINITIONS: FlowNodeDefinition[] = [
  {
    kind: "input",
    label: "Input",
    description: "Starting point, trigger, or external value.",
    category: "Core",
    sockets: [{ name: "out", direction: "output", ty: "any" }],
    defaultData: { title: "Input" },
  },
  {
    kind: "process",
    label: "Process",
    description: "A generic transformation or work step.",
    category: "Core",
    sockets: [
      { name: "in", direction: "input", ty: "any" },
      { name: "out", direction: "output", ty: "any" },
    ],
    defaultData: { title: "Process" },
  },
  {
    kind: "decision",
    label: "Decision",
    description: "Branch a flow into yes/no paths.",
    category: "Core",
    sockets: [
      { name: "in", direction: "input", ty: "any" },
      { name: "yes", direction: "output", ty: "any" },
      { name: "no", direction: "output", ty: "any" },
    ],
    defaultData: { title: "Decision" },
  },
  {
    kind: "branch",
    label: "Branch",
    description: "Fan out one input into two paths.",
    category: "Core",
    sockets: [
      { name: "in", direction: "input", ty: "any" },
      { name: "a", direction: "output", ty: "any" },
      { name: "b", direction: "output", ty: "any" },
    ],
    defaultData: { title: "Branch" },
  },
  {
    kind: "merge",
    label: "Merge",
    description: "Join two inputs into one output.",
    category: "Core",
    sockets: [
      { name: "a", direction: "input", ty: "any" },
      { name: "b", direction: "input", ty: "any" },
      { name: "out", direction: "output", ty: "any" },
    ],
    defaultData: { title: "Merge" },
  },
  {
    kind: "output",
    label: "Output",
    description: "Terminal result, sink, or destination.",
    category: "Core",
    sockets: [{ name: "in", direction: "input", ty: "any" }],
    defaultData: { title: "Output" },
  },
  {
    kind: "note",
    label: "Note",
    description: "Annotation with no graph sockets.",
    category: "Core",
    sockets: [],
    defaultData: { title: "Note" },
  },
];

const NODE_DEFINITION_BY_KIND = new Map(
  CORE_NODE_DEFINITIONS.map((definition) => [definition.kind, definition])
);

interface MountOptions {
  readOnly?: boolean;
  /** Called debounced (~500ms) after node/edge mutations. The argument
   * is a JSON-stringified `FlowJson` body — caller passes it straight
   * to `flynt_flow::parse_flow`-compatible code. */
  onChange?: (json: string) => void;
}

// ── Adapters: Flynt schema ↔ react-flow wire format ─────────────────────────

// react-flow's `Node<T>` constrains `T extends Record<string, unknown>`,
// so we intersect with that index signature. The named fields are still
// the contract — the intersection just satisfies the type variable.
type NodePayload = {
  kind: string;
  payload: Record<string, unknown>;
  sockets: SocketJson[];
} & Record<string, unknown>;

// Defensive: agents (Phase 4) may send partial nodes — missing position,
// missing data, missing sockets. We default rather than throw so a
// malformed flow renders as best-effort rather than crashing the view.
function toRfNode(n: FlowNodeJson): Node<NodePayload> {
  const pos = Array.isArray(n.position) ? n.position : [0, 0];
  return {
    id: n.id,
    type: "flynt",
    position: { x: Number(pos[0]) || 0, y: Number(pos[1]) || 0 },
    data: {
      kind: n.kind ?? "custom",
      payload: n.data ?? {},
      sockets: Array.isArray(n.sockets) ? n.sockets : [],
      label: `${n.kind ?? "custom"}: ${typeof n.data?.title === "string" ? n.data.title : n.kind ?? "custom"}`,
    },
  };
}

function toRfEdge(e: FlowEdgeJson): Edge {
  return {
    id: e.id,
    source: e.source.node,
    target: e.target.node,
    sourceHandle: e.source.socket || undefined,
    targetHandle: e.target.socket || undefined,
  };
}

// Inverse of `toRfNode` — flatten react-flow's `{x,y}` back to our `[x,y]`,
// peel the editor-only `data.payload` wrapper off, and rebuild the original
// `data: Record<string, unknown>` payload. Idempotent: round-tripping
// through `toRfNode → fromRfNode` produces the same FlowNodeJson modulo
// numeric precision (f32 ↔ f64 noise).
function fromRfNode(n: Node<NodePayload>): FlowNodeJson {
  return {
    id: n.id,
    kind: n.data.kind,
    position: [n.position.x, n.position.y],
    data: n.data.payload,
    sockets: n.data.sockets,
  };
}

function fromRfEdge(e: Edge): FlowEdgeJson {
  return {
    id: e.id,
    source: { node: e.source, socket: e.sourceHandle ?? "" },
    target: { node: e.target, socket: e.targetHandle ?? "" },
  };
}

// UUID generator. crypto.randomUUID is available in modern WebViews
// (WKWebView 14+, WebKit2GTK 2.30+) and the wry shells we ship target
// those. Fallback uses Math.random — collision probability is negligible
// for the small graph sizes we expect (<200 nodes).
function uuid(): string {
  if (typeof crypto !== "undefined" && typeof crypto.randomUUID === "function") {
    return crypto.randomUUID();
  }
  return "xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx".replace(/[xy]/g, (c) => {
    const r = (Math.random() * 16) | 0;
    const v = c === "x" ? r : (r & 0x3) | 0x8;
    return v.toString(16);
  });
}

// ── Custom node renderer ────────────────────────────────────────────────────

function FlyntNode({ data }: { data: NodePayload }) {
  const { kind, payload, sockets } = data;
  const definition = NODE_DEFINITION_BY_KIND.get(kind);
  const title =
    (typeof payload.title === "string" && payload.title) ||
    (typeof payload.name === "string" && payload.name) ||
    (typeof payload.skill === "string" && payload.skill) ||
    definition?.label || kind;

  // Group sockets by direction so input handles render on the left,
  // output handles on the right. Note nodes (no sockets) get nothing —
  // the FlowEndpoint.socket="" fallback handles connection lookup.
  const inputs = sockets.filter((s) => s.direction === "input");
  const outputs = sockets.filter((s) => s.direction === "output");

  return (
    <div
      style={{
        padding: "8px 12px",
        background: "#0e1622",
        border: "1px solid #1a3448",
        borderRadius: 6,
        color: "#e2e8f0",
        fontSize: 12,
        minWidth: 150,
        position: "relative",
        boxShadow: "0 2px 8px rgba(0,0,0,0.4)",
      }}
    >
      <div style={{ fontSize: 10, color: "#64748b", textTransform: "uppercase", letterSpacing: 0.5 }}>
        {definition?.label || kind}
      </div>
      <div style={{ fontWeight: 500, marginTop: 2 }}>{title}</div>
      {kind === "note" && typeof payload.body === "string" && payload.body && (
        <div style={{ marginTop: 6, color: "#94a3b8", fontSize: 11, lineHeight: 1.35, whiteSpace: "pre-wrap" }}>{payload.body}</div>
      )}
      {inputs.map((s, i) => (
        <React.Fragment key={`in-${s.name}`}>
          <Handle
            type="target"
            position={Position.Left}
            id={s.name}
            style={{ top: 30 + i * 18 }}
          />
          <div className="flynt-flow-socket-label input" style={{ top: 23 + i * 18 }}>{s.name}</div>
        </React.Fragment>
      ))}
      {outputs.map((s, i) => (
        <React.Fragment key={`out-${s.name}`}>
          <Handle
            type="source"
            position={Position.Right}
            id={s.name}
            style={{ top: 30 + i * 18 }}
          />
          <div className="flynt-flow-socket-label output" style={{ top: 23 + i * 18 }}>{s.name}</div>
        </React.Fragment>
      ))}
    </div>
  );
}

const NODE_TYPES = { flynt: FlyntNode };


// ── App + state management ──────────────────────────────────────────────────

function FlowApp(props: { flow: FlowJson; readOnly: boolean; onChange?: (body: FlowJson) => void }) {
  return (
    <ReactFlowProvider>
      <FlowCanvas {...props} />
    </ReactFlowProvider>
  );
}

function FlowCanvas({
  flow,
  readOnly,
  onChange,
}: {
  flow: FlowJson;
  readOnly: boolean;
  onChange?: (body: FlowJson) => void;
}) {
  // Local state seeded from the parsed flow. We don't keep `flow` itself
  // as state because react-flow operates on its own typed structures —
  // we round-trip into our schema only when emitting changes.
  const [nodes, setNodes] = React.useState<Node<NodePayload>[]>(() =>
    flow.nodes.map(toRfNode)
  );
  const [edges, setEdges] = React.useState<Edge[]>(() => flow.edges.map(toRfEdge));
  const [selectedNodeId, setSelectedNodeId] = React.useState<string | null>(null);
  const addNodeCountRef = React.useRef(flow.nodes.length);

  // Keep a ref to the current state so the debounced emitter doesn't
  // capture stale closures. React's setState batching makes "read latest
  // after change" tricky without this.
  const latestRef = React.useRef({ nodes, edges, meta: flow.meta ?? {} });
  latestRef.current = { nodes, edges, meta: flow.meta ?? {} };

  // Debounced change emit. 500ms matches the Excalidraw save cadence —
  // long enough to coalesce a drag, short enough to feel snappy on
  // discrete edits. Cmd+S triggers an immediate flush via flushEmit.
  const emitTimerRef = React.useRef<ReturnType<typeof setTimeout> | null>(null);
  const flushEmit = React.useCallback(() => {
    if (!onChange) return;
    if (emitTimerRef.current) {
      clearTimeout(emitTimerRef.current);
      emitTimerRef.current = null;
    }
    const { nodes, edges, meta } = latestRef.current;
    const body: FlowJson = {
      meta,
      nodes: nodes.map(fromRfNode),
      edges: edges.map(fromRfEdge),
    };
    // Guard host crashes — if onChange (the Rust bridge wrapper) throws
    // because the host is mid-unmount or the queue isn't writable, we
    // log and keep the editor responsive rather than letting an
    // uncaught throw kill the React tree.
    try {
      onChange(body);
    } catch (err) {
      console.error("[FlyntFlow] onChange threw", err);
    }
  }, [onChange]);

  const scheduleEmit = React.useCallback((next?: { nodes?: Node<NodePayload>[]; edges?: Edge[] }) => {
    if (next) {
      latestRef.current = {
        ...latestRef.current,
        nodes: next.nodes ?? latestRef.current.nodes,
        edges: next.edges ?? latestRef.current.edges,
      };
    }
    if (!onChange) return;
    if (emitTimerRef.current) clearTimeout(emitTimerRef.current);
    emitTimerRef.current = setTimeout(flushEmit, 500);
  }, [onChange, flushEmit]);

  // Cmd+S / Ctrl+S → immediate flush. Mirrors the Excalidraw keybind
  // so muscle memory carries across views.
  React.useEffect(() => {
    if (readOnly) return;
    function onKey(e: KeyboardEvent) {
      if ((e.metaKey || e.ctrlKey) && e.key === "s") {
        e.preventDefault();
        flushEmit();
      }
    }
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, [readOnly, flushEmit]);

  // Flush any pending debounced change on unmount so the operator's last
  // drag/edit isn't lost when navigating away mid-debounce.
  React.useEffect(() => {
    return () => {
      if (emitTimerRef.current) {
        clearTimeout(emitTimerRef.current);
        emitTimerRef.current = null;
        // Best-effort flush — onChange may already be unbound by the
        // host but the call is cheap.
        flushEmit();
      }
    };
  }, [flushEmit]);

  // Node changes: position drags, selection, dimensions. Selection-only
  // changes don't dirty the file — filter those out so we don't flood
  // disk with no-op writes when the operator clicks around.
  const onNodesChange = React.useCallback(
    (changes: NodeChange[]) => {
      setNodes((ns) => {
        const next = applyNodeChanges(changes, ns) as Node<NodePayload>[];
        const dirty = changes.some(
          (c) => c.type !== "select" && c.type !== "dimensions"
        );
        if (dirty) scheduleEmit({ nodes: next });
        return next;
      });
      const selected = changes.find((c) => c.type === "select" && c.selected);
      if (selected && "id" in selected) setSelectedNodeId(selected.id);
    },
    [scheduleEmit]
  );

  const onEdgesChange = React.useCallback(
    (changes: EdgeChange[]) => {
      setEdges((es) => {
        const next = applyEdgeChanges(changes, es);
        const dirty = changes.some((c) => c.type !== "select");
        if (dirty) scheduleEmit({ edges: next });
        return next;
      });
    },
    [scheduleEmit]
  );

  const onConnect = React.useCallback(
    (conn: Connection) => {
      const sourceNode = nodes.find((node) => node.id === conn.source);
      const targetNode = nodes.find((node) => node.id === conn.target);
      const sourceSocket = sourceNode?.data.sockets.find(
        (socket) => socket.name === (conn.sourceHandle ?? "")
      );
      const targetSocket = targetNode?.data.sockets.find(
        (socket) => socket.name === (conn.targetHandle ?? "")
      );
      if (!sourceSocket || !targetSocket || sourceSocket.direction !== "output" || targetSocket.direction !== "input") {
        return;
      }
      // react-flow generates edges without ids; we stamp a UUID so the
      // schema's id contract is satisfied and round-trips remain stable.
      const e: Edge = {
        id: uuid(),
        source: conn.source!,
        target: conn.target!,
        sourceHandle: conn.sourceHandle ?? undefined,
        targetHandle: conn.targetHandle ?? undefined,
      };
      setEdges((es) => {
        const next = addEdge(e, es);
        scheduleEmit({ edges: next });
        return next;
      });
    },
    [nodes, scheduleEmit]
  );

  const viewportPositionForNewNode = React.useCallback((index: number): { x: number; y: number } => {
    const row = Math.floor(index / 3);
    const col = index % 3;
    return {
      x: 420 + col * 260,
      y: 220 + row * 180,
    };
  }, []);

  const addNode = React.useCallback((definition: FlowNodeDefinition) => {
    const nodeId = uuid();
    const index = addNodeCountRef.current;
    addNodeCountRef.current += 1;
    const { x, y } = viewportPositionForNewNode(index);
    const node: Node<NodePayload> = {
      id: nodeId,
      type: "flynt",
      position: { x, y },
      data: {
        kind: definition.kind,
        payload: { ...(definition.defaultData ?? {}) },
        sockets: definition.sockets.map((socket) => ({ ...socket })),
        label: `${definition.label}: ${String((definition.defaultData ?? {}).title ?? definition.label)}`,
      },
    };
    const nextNodes = [...latestRef.current.nodes, node];
    latestRef.current = { ...latestRef.current, nodes: nextNodes };
    setNodes(nextNodes);
    setSelectedNodeId(null);
    scheduleEmit({ nodes: nextNodes });
  }, [scheduleEmit, viewportPositionForNewNode]);

  const addStarterFlow = React.useCallback(() => {
    const input = NODE_DEFINITION_BY_KIND.get("input")!;
    const process = NODE_DEFINITION_BY_KIND.get("process")!;
    const output = NODE_DEFINITION_BY_KIND.get("output")!;
    const inputId = uuid();
    const processId = uuid();
    const outputId = uuid();
    const makeNode = (definition: FlowNodeDefinition, id: string, x: number): Node<NodePayload> => ({
      id,
      type: "flynt",
      position: { x, y: 260 },
      data: {
        kind: definition.kind,
        payload: { ...(definition.defaultData ?? {}) },
        sockets: definition.sockets.map((socket) => ({ ...socket })),
        label: `${definition.label}: ${String((definition.defaultData ?? {}).title ?? definition.label)}`,
      },
    });
    const nextNodes = [
      makeNode(input, inputId, 420),
      makeNode(process, processId, 680),
      makeNode(output, outputId, 940),
    ];
    const nextEdges = [
      { id: uuid(), source: inputId, target: processId, sourceHandle: "out", targetHandle: "in" },
      { id: uuid(), source: processId, target: outputId, sourceHandle: "out", targetHandle: "in" },
    ];
    setNodes(nextNodes);
    setEdges(nextEdges);
    addNodeCountRef.current = nextNodes.length;
    setSelectedNodeId(null);
    scheduleEmit({ nodes: nextNodes, edges: nextEdges });
  }, [scheduleEmit]);

  const selectedNode = selectedNodeId ? nodes.find((node) => node.id === selectedNodeId) : undefined;

  const updateSelectedNodePayload = React.useCallback((patch: Record<string, unknown>) => {
    if (!selectedNodeId) return;
    setNodes((current) => {
      const next = current.map((node) => {
        if (node.id !== selectedNodeId) return node;
        return {
          ...node,
          data: {
            ...node.data,
            payload: { ...node.data.payload, ...patch },
          },
        };
      });
      scheduleEmit({ nodes: next });
      return next;
    });
  }, [selectedNodeId, scheduleEmit]);



  return (
    <div style={{ width: "100%", flex: 1, minHeight: 0, position: "relative" }}>
      <div className="flynt-flow-canvas">
        <ReactFlow
          nodes={nodes}
          edges={edges}
          nodeTypes={NODE_TYPES}
          onNodesChange={onNodesChange}
          onEdgesChange={onEdgesChange}
          onConnect={onConnect}
          onNodeClick={(_, node) => setSelectedNodeId(node.id)}
          onPaneClick={() => setSelectedNodeId(null)}
          nodesDraggable={!readOnly}
          nodesConnectable={!readOnly}
          edgesFocusable={!readOnly}
          elementsSelectable
          defaultViewport={{ x: 0, y: 0, zoom: 1 }}
          nodeOrigin={[0, 0]}
          minZoom={0.2}
          maxZoom={2}
          deleteKeyCode={readOnly ? null : ["Backspace", "Delete"]}
          style={{ width: "100%", height: "100%", background: "#020617" }}
        >
          <Background gap={20} color="#1e293b" />
          <Controls position="bottom-right" showInteractive={false} />
          <MiniMap
            position="bottom-left"
            maskColor="rgba(2,6,23,0.8)"
            style={{ background: "#0f172a", border: "1px solid #1e293b" }}
            nodeColor="#475569"
          />
        </ReactFlow>
      </div>
      {!readOnly && (
        <div className="flynt-flow-palette" onMouseDown={(event) => event.stopPropagation()}>
          <div className="flynt-flow-palette-title">Core nodes <span>{nodes.length}</span></div>
          <div className="flynt-flow-palette-grid">
            {CORE_NODE_DEFINITIONS.map((definition) => (
              <button
                key={definition.kind}
                className="flynt-flow-palette-btn"
                title={definition.description}
                onClick={() => addNode(definition)}
              >
                {definition.label}
              </button>
            ))}
          </div>
        </div>
      )}
      {!readOnly && nodes.length === 0 && (
        <div className="flynt-flow-empty">
          <h2>Build a flow</h2>
          <p>Add core nodes, then connect their handles to describe direction or dependency.</p>
          <button onClick={addStarterFlow}>Start with Input → Process → Output</button>
        </div>
      )}
      {!readOnly && selectedNode && (
        <div className="flynt-flow-inspector" onMouseDown={(event) => event.stopPropagation()}>
          <div className="flynt-flow-inspector-title">{NODE_DEFINITION_BY_KIND.get(selectedNode.data.kind)?.label || selectedNode.data.kind}</div>
          <label>
            Title
            <input
              value={typeof selectedNode.data.payload.title === "string" ? selectedNode.data.payload.title : ""}
              onChange={(event) => updateSelectedNodePayload({ title: event.target.value })}
            />
          </label>
          {selectedNode.data.kind === "note" && (
            <label>
              Body
              <textarea
                value={typeof selectedNode.data.payload.body === "string" ? selectedNode.data.payload.body : ""}
                onChange={(event) => updateSelectedNodePayload({ body: event.target.value })}
              />
            </label>
          )}
        </div>
      )}

    </div>
  );
}

// ── Public API ──────────────────────────────────────────────────────────────

interface FlyntFlowGlobal {
  mount: (elementId: string, flowJson: string, options?: MountOptions) => void;
  unmount: () => void;
  _root?: Root | null;
}

declare global {
  interface Window {
    FlyntFlow?: FlyntFlowGlobal;
  }
}

function injectStyles() {
  if (document.getElementById("flynt-flow-styles")) return;
  const style = document.createElement("style");
  style.id = "flynt-flow-styles";
  style.textContent = reactFlowCss + `
.flynt-flow-canvas { position: absolute; inset: 0; min-width: 0; min-height: 0; overflow: hidden; }
.flynt-flow-canvas .react-flow { width: 100%; height: 100%; }
.flynt-flow-palette { position: absolute; top: 12px; left: 12px; z-index: 8; width: 180px; padding: 10px; border: 1px solid #1a3448; border-radius: 10px; background: rgba(14, 22, 34, 0.92); box-shadow: 0 12px 28px rgba(0,0,0,0.35); }
.flynt-flow-palette-title { color: #6ecad8; font-size: 11px; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; margin-bottom: 8px; display: flex; justify-content: space-between; }
.flynt-flow-palette-title span { color: #607888; }
.flynt-flow-palette-grid { display: grid; gap: 6px; }
.flynt-flow-palette-btn { width: 100%; border: 1px solid #1a3448; border-radius: 7px; padding: 6px 8px; color: #c4d8e4; background: #0f172a; font-size: 12px; text-align: left; cursor: pointer; }
.flynt-flow-palette-btn:hover { border-color: #2ab4c8; color: #6ecad8; background: #131e2e; }
.flynt-flow-empty { position: absolute; z-index: 7; top: 50%; left: 50%; transform: translate(-50%, -50%); width: min(420px, 80%); border: 1px solid #1a3448; border-radius: 14px; padding: 22px; background: rgba(14, 22, 34, 0.94); color: #c4d8e4; text-align: center; box-shadow: 0 18px 50px rgba(0,0,0,0.45); }
.flynt-flow-empty h2 { margin: 0 0 8px; color: #6ecad8; font-size: 24px; }
.flynt-flow-empty p { margin: 0 0 16px; color: #607888; line-height: 1.45; }
.flynt-flow-empty button { border: 1px solid #2ab4c8; border-radius: 8px; padding: 8px 12px; color: #06080e; background: #2ab4c8; font-weight: 700; cursor: pointer; }
.flynt-flow-socket-label { position: absolute; color: #607888; font-size: 9px; line-height: 1; pointer-events: none; }
.flynt-flow-socket-label.input { left: 8px; }
.flynt-flow-socket-label.output { right: 8px; }
.flynt-flow-inspector { position: absolute; bottom: 12px; right: 12px; z-index: 8; width: 240px; padding: 10px; border: 1px solid #1a3448; border-radius: 10px; background: rgba(14, 22, 34, 0.94); box-shadow: 0 12px 28px rgba(0,0,0,0.35); color: #c4d8e4; }
.flynt-flow-inspector-title { color: #6ecad8; font-size: 11px; font-weight: 700; letter-spacing: 0.08em; text-transform: uppercase; margin-bottom: 8px; }
.flynt-flow-inspector label { display: grid; gap: 4px; color: #607888; font-size: 10px; text-transform: uppercase; letter-spacing: 0.06em; margin-top: 8px; }
.flynt-flow-inspector input, .flynt-flow-inspector textarea { width: 100%; box-sizing: border-box; border: 1px solid #1a3448; border-radius: 7px; padding: 7px 8px; background: #06080e; color: #c4d8e4; font: inherit; text-transform: none; letter-spacing: normal; }
.flynt-flow-inspector textarea { min-height: 90px; resize: vertical; }
`;
  document.head.appendChild(style);
}

const api: FlyntFlowGlobal = {
  _root: null,
  mount(elementId, flowJson, options = {}) {
    injectStyles();
    const container = document.getElementById(elementId);
    if (!container) {
      console.error(`[FlyntFlow] no element with id ${elementId}`);
      return;
    }
    let parsed: FlowJson;
    try {
      parsed = JSON.parse(flowJson);
    } catch (err) {
      console.error("[FlyntFlow] invalid flow JSON", err);
      return;
    }
    parsed.nodes = Array.isArray(parsed.nodes) ? parsed.nodes : [];
    parsed.edges = Array.isArray(parsed.edges) ? parsed.edges : [];

    const nodeIds = new Set(parsed.nodes.map((n) => n.id));
    parsed.edges = parsed.edges.filter(
      (e) =>
        e &&
        e.id &&
        e.source?.node &&
        e.target?.node &&
        nodeIds.has(e.source.node) &&
        nodeIds.has(e.target.node)
    );

    if (api._root) api._root.unmount();
    api._root = createRoot(container);
    const onChangeWrapper = options.onChange
      ? (body: FlowJson) => options.onChange!(JSON.stringify(body))
      : undefined;
    api._root.render(
      <FlowApp
        flow={parsed}
        readOnly={options.readOnly ?? false}
        onChange={onChangeWrapper}
      />
    );
  },
  unmount() {
    if (api._root) {
      try { api._root.unmount(); } catch { /* ignore */ }
      api._root = null;
    }
  },
};

window.FlyntFlow = api;
