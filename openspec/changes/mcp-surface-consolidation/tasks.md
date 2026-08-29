# Tasks

## 1. Documentation accuracy (landed ahead of this proposal)
<!-- specs: mcp-surface/consolidation -->

- [x] 1.1 Resolve README.md's self-contradiction between "MCP extension" (crate table)
      and "No MCP for integration" (design decisions).
- [x] 1.2 Correct `docs/architecture.md` and `docs/codyx-root.md` to state
      flynt-agent's actual dual-protocol behavior (Omegon's native v2 protocol by
      default, MCP via `--mcp`) without a hardcoded tool count.
- [x] 1.3 Correct `daemon_settings.rs`'s help text to stop asserting an unverified
      MCP/JSON-RPC claim for the Omegon background daemon.
- [x] 1.4 Soften `docs/ui-guide.md`'s "MCP tool"/"MCP Tools" phrasing to "agent tool" /
      "Agent Tools" where it was describing Omegon's native-protocol tool calls, not
      actual MCP traffic.

## 2. flynt-agent binary resolution
<!-- specs: mcp-surface/consolidation -->

- [x] 2.1 Add `resolve_flynt_agent_binary()` (PATH lookup) alongside the existing
      Omegon/generic-agent binary resolution helpers. Split into a pure
      `resolve_flynt_agent_binary_from_path(&str)` core so tests don't mutate the
      process-global `PATH` env var.
- [x] 2.2 Unit test: resolves a binary present on PATH (including when it's not the
      first PATH entry), returns `None` when absent.

## 3. MCP connection settings page
<!-- specs: mcp-surface/consolidation -->

- [x] 3.1 Add `SettingsPage::GeneralMcp` under `SettingsCategory::General` (state.rs),
      always visible regardless of the configured `agent_runtime`.
- [x] 3.2 Render resolved/not-found status for the flynt-agent binary.
- [x] 3.3 Render a copy-to-clipboard `mcpServers` JSON snippet (command, `["--mcp"]`,
      `FLYNT_PROJECT` env) scoped to the current project root when resolved; a build
      hint when not.
- [x] 3.4 Manual verification: confirmed both UI states live (not-found, then found
      after building `flynt-agent --release` and placing it on PATH — status line and
      generated JSON updated correctly on page revisit). Verified the exact generated
      command/args/env by piping a real `initialize` + `tools/list` JSON-RPC handshake
      into `flynt-agent --mcp` directly — got back valid `serverInfo` and 66 real tool
      definitions with schemas, proving the config Settings hands operators actually
      works, not just that it renders.

## 4. Spec enforcement
<!-- specs: mcp-surface/consolidation -->

- [x] 4.1 Delta spec landed at `specs/mcp-surface/consolidation.md` so future changes
      to `daemon_settings.rs` or the README/docs are reviewable against an explicit
      requirement, not just convention.
