# MCP Surface Consolidation

## Intent

Flynt already has a working MCP server — `flynt-agent --mcp` exposes dozens of project
tools (documents, tasks, graph, drawings, diagrams, forge/engagement tracking) over
stdio to any MCP client. It was undiscoverable and inconsistently documented: README.md
contradicted itself about whether MCP existed at all, three other docs cited a stale
tool count, and the unrelated Omegon background daemon's settings copy claimed to host
"MCP / JSON-RPC endpoints" that Flynt's own code never verifies. Make `flynt-agent
--mcp` the single, accurately-documented, discoverable way any MCP client integrates
with a Flynt project, and draw a hard boundary so the daemon's copy can't regress into
re-claiming a protocol Flynt doesn't own.

## Scope

- A `flynt-agent` binary resolver (PATH lookup), mirroring the trust model already used
  for the Generic ACP runtime's command resolution.
- A new, always-visible Settings surface showing MCP connection status and a
  copy-ready client config snippet (command/args/env) for the current project.
- A spec requirement naming `flynt-agent --mcp` as the canonical, Flynt-verified MCP
  entrypoint, and prohibiting the Omegon background daemon's UI copy from asserting
  MCP compliance Flynt hasn't verified.
- A spec requirement that documentation describing flynt-agent's tool surface must not
  hardcode a tool count (already corrected directly; codified here so it doesn't
  regress).

## Out of scope

- A socket/HTTP MCP transport for flynt-agent — stdio subprocess is the standard MCP
  client pattern and matches every reference client config (Claude Desktop, Cursor,
  Zed, Claude Code all spawn a `command`).
- Verifying or reimplementing whatever protocol the Omegon background daemon speaks on
  its configured port — that binary is Omegon's, not Flynt's, to specify.
- New flynt-agent tools or changes to existing tool behavior.
- Auto-generating tool documentation from source (reasonable follow-up, not required
  for a coherent surface today).
