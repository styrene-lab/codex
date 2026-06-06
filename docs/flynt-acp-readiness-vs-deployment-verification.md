---
id: flynt-acp-readiness-vs-deployment-verification
title: "Separate ACP readiness from Flynt deployment verification"
status: exploring
tags: [acp, omegon, runtime, diagnostics]
open_questions:
  - "[assumption] Omegon exposes a stable ACP extension RPC route that Flynt can call to invoke the flynt extension initialize method after session connect."
  - "Which Omegon ACP method should Flynt use for the active deployment probe: direct namespaced extension method, extensions/call, tools/call, or a new runtime surface?"
  - "Should failed deployment verification block only Flynt-specific tools/actions, or also disable the general prompt box when the mismatch is severe?"
dependencies: []
related: []
---

# Separate ACP readiness from Flynt deployment verification

## Overview

Clarify and implement Flynt's runtime diagnostics so generic ACP transport/session readiness is distinct from Flynt-specific extension/deployment contract verification. Add an active post-connect deployment probe instead of relying on passive initialize metadata observation.

## Research

### Current agent rail behavior

Current Flynt agent rail computes a single preflight from `DeploymentDiagnostic` plus CLI probe. `preflight_is_blocked` only blocks `DeploymentStatus::Blocked` or CLI incompatible. The recent stabilization commit reframed `DeploymentStatus::Unknown` plus CLI ready as usable/not verified rather than warning/blocking. Evidence: `crates/flynt-app/src/components/agent_rail.rs` AgentPreflightCard and preflight_is_blocked.

### Current deployment classifier

Current deployment classifier treats missing extension initialize metadata as `DeploymentStatus::Unknown` only when the local deployment manifest is otherwise OK. It checks Flynt-specific extension data: required_profile, project_root, capability_contract_version, surface_guide_version. Evidence: `crates/flynt-app/src/omegon_deployment_diagnostics.rs::classify_deployment`.

### Flynt extension metadata source

The Flynt extension already implements an `initialize` RPC returning the needed metadata under `extension_info` and `_meta.flynt.extension_info`: name, version, sdk/runtime versions, project_root, required_profile, surface_guide_version, capability_contract_version, and policy.memory_scope=project. Evidence: `crates/flynt-agent/src/extension.rs` initialize handler.

## Open Questions

- [assumption] Omegon exposes a stable ACP extension RPC route that Flynt can call to invoke the flynt extension initialize method after session connect.
- Which Omegon ACP method should Flynt use for the active deployment probe: direct namespaced extension method, extensions/call, tools/call, or a new runtime surface?
- Should failed deployment verification block only Flynt-specific tools/actions, or also disable the general prompt box when the mismatch is severe?
