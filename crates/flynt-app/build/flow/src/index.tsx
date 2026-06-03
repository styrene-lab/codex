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
  useReactFlow,
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

// ── Kind → accent color mapping ─────────────────────────────────────────────
// Alpharius semantic palette: each node kind gets a distinct accent
// border-top so the operator can scan the canvas at a glance.

const KIND_ACCENT: Record<string, string> = {
  input:    "#1ab878", // start — hydra emerald
  output:   "#b89020", // end — tarnished brass
  process:  "#2ab4c8", // primary — ceramite teal
  decision: "#c83030", // decision — blood red
  branch:   "#c86418", // warning — hot metal
  merge:    "#6060c0", // AI/merge — indigo
  note:     "#607888", // muted — annotation grey
};

function FlyntNode({ data, selected }: { data: NodePayload; selected?: boolean }) {
  const { kind, payload, sockets } = data;
  const definition = NODE_DEFINITION_BY_KIND.get(kind);
  const accent = KIND_ACCENT[kind] ?? "#2ab4c8";
  const title =
    (typeof payload.title === "string" && payload.title) ||
    (typeof payload.name === "string" && payload.name) ||
    (typeof payload.skill === "string" && payload.skill) ||
    definition?.label || kind;

  const inputs = sockets.filter((s) => s.direction === "input");
  const outputs = sockets.filter((s) => s.direction === "output");
  const socketRows = Math.max(inputs.length, outputs.length);

  return (
    <div className="flynt-node" data-selected={selected ? "" : undefined} style={{ borderTopColor: accent }}>
      {/* Header */}
      <div className="flynt-node-kind" style={{ color: accent }}>
        {definition?.label || kind}
      </div>
      <div className="flynt-node-title">{title}</div>

      {/* Note body */}
      {kind === "note" && typeof payload.body === "string" && payload.body && (
        <div className="flynt-node-body">{payload.body}</div>
      )}

      {/* Sockets — rendered as rows so handles align naturally */}
      {socketRows > 0 && (
        <div className="flynt-node-sockets">
          {Array.from({ length: socketRows }, (_, i) => {
            const inp = inputs[i];
            const out = outputs[i];
            return (
              <div key={i} className="flynt-node-socket-row">
                <div className="flynt-node-socket-cell left">
                  {inp && (
                    <>
                      <Handle type="target" position={Position.Left} id={inp.name}
                        className="flynt-handle" />
                      <span className="flynt-socket-name">{inp.name}</span>
                    </>
                  )}
                </div>
                <div className="flynt-node-socket-cell right">
                  {out && (
                    <>
                      <span className="flynt-socket-name">{out.name}</span>
                      <Handle type="source" position={Position.Right} id={out.name}
                        className="flynt-handle" />
                    </>
                  )}
                </div>
              </div>
            );
          })}
        </div>
      )}
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

  // Fab popover + context menu state
  const [fabOpen, setFabOpen] = React.useState(false);
  const [contextMenu, setContextMenu] = React.useState<{ x: number; y: number; flowPos: { x: number; y: number } } | null>(null);
  const reactFlowInstance = useReactFlow();

  // Undo: single-level snapshot before destructive ops
  const [undoSnapshot, setUndoSnapshot] = React.useState<{ nodes: Node<NodePayload>[]; edges: Edge[]; label: string } | null>(null);
  const undoRef = React.useRef<() => void>(() => {});

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
      if ((e.metaKey || e.ctrlKey) && e.key === "z" && !e.shiftKey) {
        e.preventDefault();
        undoRef.current();
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

  const viewportCenterPosition = React.useCallback((): { x: number; y: number } => {
    // Place new nodes at the center of the current viewport, with a
    // small random offset so rapid sequential adds don't stack exactly.
    try {
      const container = document.querySelector('.flynt-flow-canvas');
      if (container) {
        const rect = container.getBoundingClientRect();
        const center = reactFlowInstance.screenToFlowPosition({
          x: rect.left + rect.width / 2,
          y: rect.top + rect.height / 2,
        });
        const jitter = () => (Math.random() - 0.5) * 60;
        return { x: Math.round(center.x + jitter()), y: Math.round(center.y + jitter()) };
      }
    } catch { /* fallback below */ }
    return { x: 200, y: 200 };
  }, [reactFlowInstance]);

  const addNode = React.useCallback((definition: FlowNodeDefinition, position?: { x: number; y: number }) => {
    const nodeId = uuid();
    const pos = position ?? viewportCenterPosition();
    addNodeCountRef.current += 1;
    const node: Node<NodePayload> = {
      id: nodeId,
      type: "flynt",
      position: pos,
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
    setSelectedNodeId(nodeId);
    scheduleEmit({ nodes: nextNodes });
  }, [scheduleEmit, viewportCenterPosition]);

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

  // Close popover / context menu on outside click or Escape
  React.useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") { setFabOpen(false); setContextMenu(null); }
    }
    document.addEventListener("keydown", onKey);
    return () => document.removeEventListener("keydown", onKey);
  }, []);

  // Context menu handler — right-click on the canvas pane
  const onPaneContextMenu = React.useCallback((event: React.MouseEvent | MouseEvent) => {
    event.preventDefault();
    const clientX = "clientX" in event ? event.clientX : 0;
    const clientY = "clientY" in event ? event.clientY : 0;
    const flowPos = reactFlowInstance.screenToFlowPosition({ x: clientX, y: clientY });
    setContextMenu({ x: clientX, y: clientY, flowPos });
    setFabOpen(false);
  }, [reactFlowInstance]);

  // Helper: add node from a menu and close it
  const addNodeFromMenu = React.useCallback((definition: FlowNodeDefinition, position?: { x: number; y: number }) => {
    addNode(definition, position);
    setFabOpen(false);
    setContextMenu(null);
  }, [addNode]);

  // Action: delete selected nodes and edges
  const deleteSelected = React.useCallback(() => {
    const selectedNodes = nodes.filter((n) => n.selected);
    const selectedEdges = edges.filter((e) => e.selected);
    if (selectedNodes.length === 0 && selectedEdges.length === 0) return;
    setUndoSnapshot({ nodes: [...nodes], edges: [...edges], label: `Delete ${selectedNodes.length + selectedEdges.length} element${selectedNodes.length + selectedEdges.length > 1 ? "s" : ""}` });
    reactFlowInstance.deleteElements({ nodes: selectedNodes, edges: selectedEdges });
    setSelectedNodeId(null);
    setContextMenu(null);
    setFabOpen(false);
  }, [nodes, edges, reactFlowInstance]);

  // Action: select all
  const selectAll = React.useCallback(() => {
    setNodes((ns) => ns.map((n) => ({ ...n, selected: true })));
    setEdges((es) => es.map((e) => ({ ...e, selected: true })));
    setContextMenu(null);
  }, []);

  // Action: clear all
  const clearAll = React.useCallback(() => {
    if (nodes.length === 0 && edges.length === 0) return;
    setUndoSnapshot({ nodes: [...nodes], edges: [...edges], label: `Clear ${nodes.length} nodes` });
    setNodes([]);
    setEdges([]);
    latestRef.current = { ...latestRef.current, nodes: [], edges: [] };
    setSelectedNodeId(null);
    addNodeCountRef.current = 0;
    scheduleEmit({ nodes: [], edges: [] });
    setContextMenu(null);
    setFabOpen(false);
  }, [nodes, edges, scheduleEmit]);

  // Action: duplicate selected
  const duplicateSelected = React.useCallback(() => {
    const sel = nodes.filter((n) => n.selected);
    if (sel.length === 0) return;
    const newNodes = sel.map((n) => ({
      ...n,
      id: uuid(),
      position: { x: n.position.x + 40, y: n.position.y + 40 },
      selected: true,
      data: { ...n.data, payload: { ...n.data.payload } },
    }));
    // Deselect originals, add clones
    setNodes((ns) => {
      const next = ns.map((n) => ({ ...n, selected: false })).concat(newNodes);
      scheduleEmit({ nodes: next });
      return next;
    });
    addNodeCountRef.current += newNodes.length;
    setContextMenu(null);
  }, [nodes, scheduleEmit]);

  // Action: fit view
  const doFitView = React.useCallback(() => {
    reactFlowInstance.fitView({ padding: 0.15, duration: 300 });
    setContextMenu(null);
    setFabOpen(false);
  }, [reactFlowInstance]);

  // Action: undo last destructive operation
  const undo = React.useCallback(() => {
    if (!undoSnapshot) return;
    setNodes(undoSnapshot.nodes);
    setEdges(undoSnapshot.edges);
    latestRef.current = { ...latestRef.current, nodes: undoSnapshot.nodes, edges: undoSnapshot.edges };
    addNodeCountRef.current = undoSnapshot.nodes.length;
    scheduleEmit({ nodes: undoSnapshot.nodes, edges: undoSnapshot.edges });
    setUndoSnapshot(null);
    setSelectedNodeId(null);
    setContextMenu(null);
    setFabOpen(false);
  }, [undoSnapshot, scheduleEmit]);
  undoRef.current = undo;



  return (
    <div style={{ width: "100%", flex: 1, minHeight: 0, position: "relative" }}
         onClick={() => { setFabOpen(false); setContextMenu(null); }}>
      <div className="flynt-flow-canvas">
        <ReactFlow
          nodes={nodes}
          edges={edges}
          nodeTypes={NODE_TYPES}
          onNodesChange={onNodesChange}
          onEdgesChange={onEdgesChange}
          onConnect={onConnect}
          onNodeClick={(_, node) => { setSelectedNodeId(node.id); setContextMenu(null); }}
          onPaneClick={() => { setSelectedNodeId(null); setContextMenu(null); setFabOpen(false); }}
          onPaneContextMenu={readOnly ? undefined : onPaneContextMenu}
          nodesDraggable={!readOnly}
          nodesConnectable={!readOnly}
          edgesFocusable={!readOnly}
          elementsSelectable
          fitView
          fitViewOptions={{ padding: 0.15, maxZoom: 1 }}
          nodeOrigin={[0, 0]}
          minZoom={0.15}
          maxZoom={2.5}
          proOptions={{ hideAttribution: true }}
          deleteKeyCode={readOnly ? null : ["Backspace", "Delete"]}
          defaultEdgeOptions={{
            style: { stroke: "#1a3448", strokeWidth: 2 },
            type: "smoothstep",
          }}
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

      {/* ── Fab button ──────────────────────────────────────── */}
      {!readOnly && (
        <button
          className={`flynt-fab ${fabOpen ? "open" : ""}`}
          onClick={(e) => { e.stopPropagation(); setFabOpen(!fabOpen); setContextMenu(null); }}
          title="Add node"
        >
          <svg width="20" height="20" viewBox="0 0 20 20" fill="none">
            <path d="M10 4v12M4 10h12" stroke="currentColor" strokeWidth="2" strokeLinecap="round"/>
          </svg>
        </button>
      )}

      {/* ── Fab popover ─────────────────────────────────────── */}
      {!readOnly && fabOpen && (
        <div className="flynt-fab-popover" onClick={(e) => e.stopPropagation()}>
          <div className="flynt-menu-section">
            <div className="flynt-menu-section-title">Add node</div>
            {CORE_NODE_DEFINITIONS.map((definition) => (
              <button key={definition.kind} className="flynt-menu-item"
                onClick={() => addNodeFromMenu(definition)}>
                <span className="flynt-menu-item-dot" style={{ background: KIND_ACCENT[definition.kind] ?? "#2ab4c8" }} />
                <span className="flynt-menu-item-label">{definition.label}</span>
                <span className="flynt-menu-item-desc">{definition.description}</span>
              </button>
            ))}
          </div>
          {nodes.length === 0 && (
            <div className="flynt-menu-section">
              <div className="flynt-menu-section-title">Quick start</div>
              <button className="flynt-menu-item" onClick={() => { addStarterFlow(); setFabOpen(false); }}>
                <span className="flynt-menu-item-dot" style={{ background: "#2ab4c8" }} />
                <span className="flynt-menu-item-label">Input → Process → Output</span>
                <span className="flynt-menu-item-desc">Starter three-node flow</span>
              </button>
            </div>
          )}
          <div className="flynt-menu-section">
            {undoSnapshot && (
              <button className="flynt-menu-item" onClick={undo}>
                <span className="flynt-menu-item-label">Undo</span>
                <span className="flynt-menu-item-shortcut">⌘Z</span>
              </button>
            )}
            <button className="flynt-menu-item" onClick={doFitView}>
              <span className="flynt-menu-item-label">Fit view</span>
            </button>
            {nodes.length > 0 && (
              <button className="flynt-menu-item flynt-menu-item-danger" onClick={clearAll}>
                <span className="flynt-menu-item-label">Clear all</span>
                <span className="flynt-menu-item-shortcut">{nodes.length} nodes</span>
              </button>
            )}
          </div>
        </div>
      )}

      {/* ── Context menu ────────────────────────────────────── */}
      {!readOnly && contextMenu && (() => {
        const hasSelection = nodes.some((n) => n.selected) || edges.some((e) => e.selected);
        const selectedCount = nodes.filter((n) => n.selected).length + edges.filter((e) => e.selected).length;
        return (
          <div className="flynt-context-menu"
            style={{ left: contextMenu.x, top: contextMenu.y }}
            onClick={(e) => e.stopPropagation()}>

            {/* Selection actions — only when something is selected */}
            {hasSelection && (
              <div className="flynt-menu-section">
                <button className="flynt-menu-item" onClick={duplicateSelected}>
                  <span className="flynt-menu-item-label">Duplicate</span>
                  <span className="flynt-menu-item-shortcut">{selectedCount} selected</span>
                </button>
                <button className="flynt-menu-item flynt-menu-item-danger" onClick={deleteSelected}>
                  <span className="flynt-menu-item-label">Delete</span>
                  <span className="flynt-menu-item-shortcut">⌫</span>
                </button>
              </div>
            )}

            {/* Add node */}
            <div className="flynt-menu-section">
              <div className="flynt-menu-section-title">Add node</div>
              {CORE_NODE_DEFINITIONS.map((definition) => (
                <button key={definition.kind} className="flynt-menu-item"
                  onClick={() => addNodeFromMenu(definition, contextMenu.flowPos)}>
                  <span className="flynt-menu-item-dot" style={{ background: KIND_ACCENT[definition.kind] ?? "#2ab4c8" }} />
                  <span className="flynt-menu-item-label">{definition.label}</span>
                </button>
              ))}
            </div>

            {/* Canvas actions */}
            <div className="flynt-menu-section">
              {undoSnapshot && (
                <button className="flynt-menu-item" onClick={undo}>
                  <span className="flynt-menu-item-label">Undo</span>
                  <span className="flynt-menu-item-shortcut">⌘Z</span>
                </button>
              )}
              <button className="flynt-menu-item" onClick={selectAll}>
                <span className="flynt-menu-item-label">Select all</span>
                <span className="flynt-menu-item-shortcut">⌘A</span>
              </button>
              <button className="flynt-menu-item" onClick={doFitView}>
                <span className="flynt-menu-item-label">Fit view</span>
              </button>
              {nodes.length > 0 && (
                <button className="flynt-menu-item flynt-menu-item-danger" onClick={clearAll}>
                  <span className="flynt-menu-item-label">Clear all</span>
                  <span className="flynt-menu-item-shortcut">{nodes.length} nodes</span>
                </button>
              )}
            </div>
          </div>
        );
      })()}

      {/* ── Empty state ─────────────────────────────────────── */}
      {!readOnly && nodes.length === 0 && !fabOpen && (
        <div className="flynt-flow-empty">
          <h2>Build a flow</h2>
          <p>Click <strong>+</strong> or right-click the canvas to add nodes.</p>
        </div>
      )}

      {/* ── Node inspector ──────────────────────────────────── */}
      {!readOnly && selectedNode && (
        <div className="flynt-flow-inspector" onClick={(e) => e.stopPropagation()} onMouseDown={(e) => e.stopPropagation()}>
          <div className="flynt-flow-inspector-header">
            <span className="flynt-flow-inspector-kind" style={{
              color: KIND_ACCENT[selectedNode.data.kind] ?? "#2ab4c8",
              borderColor: KIND_ACCENT[selectedNode.data.kind] ?? "#2ab4c8",
            }}>
              {NODE_DEFINITION_BY_KIND.get(selectedNode.data.kind)?.label || selectedNode.data.kind}
            </span>
            <button className="flynt-flow-inspector-close" onClick={() => setSelectedNodeId(null)}>×</button>
          </div>
          <label>
            <span className="flynt-flow-inspector-label">Title</span>
            <input
              value={typeof selectedNode.data.payload.title === "string" ? selectedNode.data.payload.title : ""}
              onChange={(event) => updateSelectedNodePayload({ title: event.target.value })}
            />
          </label>
          {selectedNode.data.kind === "note" && (
            <label>
              <span className="flynt-flow-inspector-label">Body</span>
              <textarea
                value={typeof selectedNode.data.payload.body === "string" ? selectedNode.data.payload.body : ""}
                onChange={(event) => updateSelectedNodePayload({ body: event.target.value })}
              />
            </label>
          )}
          <div className="flynt-flow-inspector-meta">
            {selectedNode.data.sockets.length > 0 && (
              <span>{selectedNode.data.sockets.filter(s => s.direction === "input").length} in · {selectedNode.data.sockets.filter(s => s.direction === "output").length} out</span>
            )}
          </div>
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
.flynt-fab { position: absolute; top: 12px; left: 12px; z-index: 10; width: 40px; height: 40px; border-radius: 12px; border: 1px solid #1a3448; background: #0e1622; color: #607888; cursor: pointer; display: flex; align-items: center; justify-content: center; transition: all 0.15s; box-shadow: 0 2px 8px rgba(0,0,0,0.3); }
.flynt-fab:hover { color: #c4d8e4; border-color: #2ab4c8; background: #131e2e; }
.flynt-fab.open { color: #2ab4c8; border-color: #2ab4c8; background: #131e2e; transform: rotate(45deg); }
.flynt-fab-popover { position: absolute; top: 58px; left: 12px; z-index: 10; width: 260px; max-height: 70vh; overflow-y: auto; background: rgba(14, 22, 34, 0.97); border: 1px solid #1a3448; border-radius: 10px; box-shadow: 0 12px 32px rgba(0,0,0,0.5); backdrop-filter: blur(8px); }
.flynt-context-menu { position: fixed; z-index: 20; width: 220px; background: rgba(14, 22, 34, 0.97); border: 1px solid #1a3448; border-radius: 10px; box-shadow: 0 12px 32px rgba(0,0,0,0.5); backdrop-filter: blur(8px); }
.flynt-menu-section { padding: 6px 0; }
.flynt-menu-section + .flynt-menu-section { border-top: 1px solid #1a3448; }
.flynt-menu-section-title { padding: 6px 14px 4px; font-size: 9px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.08em; color: #475569; }
.flynt-menu-item { display: flex; align-items: flex-start; gap: 10px; width: 100%; padding: 7px 14px; border: none; background: transparent; color: #c4d8e4; cursor: pointer; text-align: left; font-size: 12px; line-height: 1.3; }
.flynt-menu-item:hover { background: rgba(42, 180, 200, 0.08); }
.flynt-menu-item-dot { width: 8px; height: 8px; border-radius: 50%; flex-shrink: 0; margin-top: 3px; }
.flynt-menu-item-label { font-weight: 600; white-space: nowrap; }
.flynt-menu-item-desc { color: #475569; font-size: 11px; flex: 1; }
.flynt-menu-item-shortcut { color: #475569; font-size: 10px; margin-left: auto; padding-left: 12px; white-space: nowrap; }
.flynt-menu-item-danger .flynt-menu-item-label { color: #c83030; }
.flynt-menu-item-danger:hover { background: rgba(200, 48, 48, 0.1); }
.flynt-menu-item-danger:hover .flynt-menu-item-label { color: #ef4444; }
.flynt-flow-empty { position: absolute; z-index: 7; top: 50%; left: 50%; transform: translate(-50%, -50%); width: min(420px, 80%); border: 1px solid #1a3448; border-radius: 14px; padding: 22px; background: rgba(14, 22, 34, 0.94); color: #c4d8e4; text-align: center; box-shadow: 0 18px 50px rgba(0,0,0,0.45); }
.flynt-flow-empty h2 { margin: 0 0 8px; color: #6ecad8; font-size: 24px; }
.flynt-flow-empty p { margin: 0 0 16px; color: #607888; line-height: 1.45; }
.flynt-flow-empty button { border: 1px solid #2ab4c8; border-radius: 8px; padding: 8px 12px; color: #06080e; background: #2ab4c8; font-weight: 700; cursor: pointer; }
.flynt-flow-socket-label { position: absolute; color: #607888; font-size: 9px; line-height: 1; pointer-events: none; font-weight: 500; letter-spacing: 0.3px; }
.flynt-flow-socket-label.input { left: 12px; }
.flynt-flow-socket-label.output { right: 12px; text-align: right; }
.flynt-node { background: #0e1622; border: 1px solid #1a3448; border-top: 2px solid #2ab4c8; border-radius: 8px; min-width: 160px; box-shadow: 0 2px 8px rgba(0,0,0,0.4); transition: border-color 0.15s, box-shadow 0.15s; }
.flynt-node[data-selected] { border-color: #2ab4c8; box-shadow: 0 0 12px rgba(42,180,200,0.2), 0 2px 8px rgba(0,0,0,0.5); }
.flynt-node-kind { padding: 8px 12px 0; font-size: 9px; font-weight: 700; text-transform: uppercase; letter-spacing: 1px; }
.flynt-node-title { padding: 2px 12px 8px; font-size: 13px; font-weight: 600; color: #e2e8f0; }
.flynt-node-body { padding: 0 12px 8px; color: #94a3b8; font-size: 11px; line-height: 1.4; white-space: pre-wrap; }
.flynt-node-sockets { border-top: 1px solid #1a3448; padding: 6px 0; }
.flynt-node-socket-row { display: flex; justify-content: space-between; align-items: center; min-height: 22px; padding: 0 12px; position: relative; }
.flynt-node-socket-cell { display: flex; align-items: center; gap: 6px; }
.flynt-node-socket-cell.right { margin-left: auto; }
.flynt-socket-name { font-size: 10px; color: #607888; font-weight: 500; }
.flynt-node[data-selected] .flynt-socket-name { color: #94a3b8; }
.flynt-handle { width: 9px !important; height: 9px !important; background: #0e1622 !important; border: 2px solid #2ab4c8 !important; }
.flynt-handle:hover { background: #2ab4c8 !important; }
.react-flow__edge-path { transition: stroke 0.15s; }
.react-flow__edge.selected .react-flow__edge-path { stroke: #2ab4c8 !important; stroke-width: 2.5px !important; }
.react-flow__controls { background: #0e1622 !important; border: 1px solid #1a3448 !important; border-radius: 8px !important; overflow: hidden !important; }
.react-flow__controls-button { background: transparent !important; border-bottom: 1px solid #1a3448 !important; color: #607888 !important; fill: #607888 !important; }
.react-flow__controls-button:hover { background: #131e2e !important; color: #c4d8e4 !important; fill: #c4d8e4 !important; }
.react-flow__controls-button:last-child { border-bottom: none !important; }
.react-flow__controls-button svg { fill: inherit; }
.react-flow__attribution { display: none !important; }
.flynt-flow-inspector { position: absolute; bottom: 12px; right: 12px; z-index: 8; width: 260px; border: 1px solid #1a3448; border-radius: 10px; background: rgba(14, 22, 34, 0.96); box-shadow: 0 12px 28px rgba(0,0,0,0.45); color: #c4d8e4; backdrop-filter: blur(8px); }
.flynt-flow-inspector-header { display: flex; justify-content: space-between; align-items: center; padding: 10px 12px 8px; border-bottom: 1px solid #1a3448; }
.flynt-flow-inspector-kind { font-size: 10px; font-weight: 700; text-transform: uppercase; letter-spacing: 0.08em; border: 1px solid; border-radius: 4px; padding: 2px 8px; }
.flynt-flow-inspector-close { background: none; border: none; color: #607888; font-size: 18px; cursor: pointer; padding: 0 4px; line-height: 1; }
.flynt-flow-inspector-close:hover { color: #c4d8e4; }
.flynt-flow-inspector label { display: grid; gap: 4px; padding: 8px 12px 0; }
.flynt-flow-inspector-label { color: #607888; font-size: 10px; font-weight: 600; text-transform: uppercase; letter-spacing: 0.06em; }
.flynt-flow-inspector input, .flynt-flow-inspector textarea { width: 100%; box-sizing: border-box; border: 1px solid #1a3448; border-radius: 6px; padding: 7px 10px; background: #06080e; color: #c4d8e4; font: inherit; font-size: 12px; }
.flynt-flow-inspector input:focus, .flynt-flow-inspector textarea:focus { outline: none; border-color: #2ab4c8; }
.flynt-flow-inspector textarea { min-height: 80px; resize: vertical; }
.flynt-flow-inspector-meta { padding: 8px 12px 10px; color: #475569; font-size: 10px; }
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
