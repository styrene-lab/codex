# MCP Surface Consolidation — Delta Spec

## ADDED Requirements

### Requirement: flynt-agent --mcp is the canonical MCP entrypoint

Flynt MUST document `flynt-agent --mcp` as the single, canonical way for an external
MCP client to reach Flynt project tools. No other component may claim to be an MCP
entrypoint, or a specific tool count, unless Flynt's own code verifies the claim.

#### Scenario: Documentation states the real protocol split
Given an operator reads README.md, docs/architecture.md, or docs/codyx-root.md
When they look for how Omegon or an MCP client reaches flynt-agent's tools
Then the docs state that Omegon uses flynt-agent's native v2 protocol by default
And the docs state that any MCP client reaches the same tools via `flynt-agent --mcp`
And no doc asserts a specific tool count

#### Scenario: A UI surface never claims unverified MCP compliance
Given the Omegon background daemon settings panel describes what the daemon hosts
When Flynt's own code cannot verify the protocol spoken on the daemon's port
Then the panel's copy MUST NOT claim the daemon hosts MCP or JSON-RPC endpoints
And the panel MUST point operators at `flynt-agent --mcp` for a Flynt-verified surface

### Requirement: Operators can discover and configure the MCP surface from Settings

Flynt MUST provide a Settings surface, visible regardless of the configured agent
runtime, that shows whether the `flynt-agent` binary is reachable and, if so, a
ready-to-use MCP client config for the current project.

#### Scenario: flynt-agent is on PATH
Given `flynt-agent` resolves on the operator's PATH
When the operator opens the MCP settings page
Then Flynt shows the resolved binary path
And Flynt shows a copy-to-clipboard `mcpServers` JSON snippet naming that path,
  `["--mcp"]`, and `FLYNT_PROJECT` set to the current project root

#### Scenario: flynt-agent is not found
Given `flynt-agent` does not resolve on the operator's PATH
When the operator opens the MCP settings page
Then Flynt shows a not-found state
And Flynt shows a hint to build it with `cargo build --release -p flynt-agent`
