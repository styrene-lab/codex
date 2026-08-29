# Design: MCP Surface Consolidation

## Overview

`flynt-agent --mcp` already works: it runs `omegon_extension::mcp_shim::serve_mcp`
over stdio and reaches the same tool handlers (`crates/flynt-agent/src/extension.rs`)
that Omegon's native v2 protocol uses by default. Nothing about the tool
implementations needs to change. What's missing is discoverability — no UI surface
tells an operator the feature exists or gives them a config snippet to paste into an
MCP client — and a boundary that stops a second, unrelated feature (the Omegon
background daemon) from re-asserting an MCP claim Flynt can't verify. This change adds
the discoverability surface and the boundary; it does not touch flynt-agent's protocol
handling.

## Components

### flynt-agent binary resolution

New resolver, likely `crates/flynt-app/src/bootstrap.rs` alongside
`resolve_omegon_binary`, or a small sibling module: `resolve_flynt_agent_binary() ->
Option<PathBuf>`. v1 is PATH lookup only — no override, no bundled-resource search, no
versions directory. This matches the trust model `resolve_agent_spawn`'s `Generic` arm
already uses for arbitrary ACP commands (`crates/flynt-app/src/components/agent_rail.rs`):
trust the OS to resolve a bare command name, don't hand-roll a search Cargo/the OS
already does better.

### MCP connection settings page

New `SettingsPage::GeneralMcp` (`crates/flynt-app/src/state.rs`), under
`SettingsCategory::General` — same placement precedent as `SettingsPage::GeneralRuntime`
(added for the ACP-runtime toggle), but this page is Flynt's *server*-facing surface
and is unrelated to which agent runtime is configured, so it carries no
`uses_omegon()`-style gating.

Renders:
- Resolved/not-found status for the `flynt-agent` binary.
- When resolved: a read-only, copy-to-clipboard JSON snippet —
  `{"mcpServers": {"flynt": {"command": "<resolved path>", "args": ["--mcp"], "env":
  {"FLYNT_PROJECT": "<project root>"}}}}` — scoped to the current project root.
  Copy-to-clipboard reuses the existing `document::eval("navigator.clipboard.writeText(...)")`
  pattern already used in `crates/flynt-app/src/components/identity_settings.rs:174`.
- When not resolved: a short hint pointing at `cargo build --release -p flynt-agent`,
  consistent with the project's current dev-focused build story (flynt-agent isn't
  bundled inside the app today).

### Daemon copy boundary

`crates/flynt-app/src/components/daemon_settings.rs`'s help text was corrected
directly (ahead of this proposal) to stop claiming the background daemon "hosts MCP /
JSON-RPC endpoints" — that daemon spawns the external `omegon` binary with nulled
stdio on a TCP port Flynt only probes for raw connectivity, never protocol-verifies.
This file is the enforcement point for the spec requirement below: future edits to
that copy must not reintroduce an MCP claim Flynt hasn't verified.

## Tradeoffs

- PATH-only binary resolution means an operator who built `flynt-agent` into
  `target/release/` but didn't add it to PATH sees "not found." Acceptable for now
  since nothing bundles flynt-agent into the app; revisit if/when it starts shipping
  as a bundled resource alongside the app binary.
- Tool-count accuracy is enforced by *not stating a count* in docs, not by
  auto-generating one from source. A doc-generation script off `tools/list` is a
  reasonable follow-up but adds CI surface this slice doesn't need.
