---
id: flynt-native-invocation-contract
title: "Flynt native invocation and capture contract"
status: decided
priority: 1
parent: flynt-root
tags: [native, macos, ios, deep-links, capture, finder, shortcuts, styrene, omegon]
dependencies: [flynt-omegon-deployment-contract, flynt-omegon-artifact-scope]
related: [artifact-actions, project-navigation-command-layer, apple-notes-import, flynt-host-actions-platform]
open_questions: []
---

# Flynt native invocation and capture contract

## Overview

Give Flynt first-class Apple-platform affordances without duplicating responsibilities owned by Omegon, shared Styrene infrastructure, or domain packages. Native OS integrations are adapters into typed Flynt project actions. They are not a second automation runtime, identity system, secret store, project registry, or agent protocol.

This node is binding architecture for deep links, Finder actions, Share extensions, App Intents, Shortcuts, and related native ingress.

## Existing constraints

- The decided [[flynt-omegon-deployment-contract]] assigns workspace UI, project roots, documents, tasks, artifacts, and the ACP panel to Flynt. Omegon owns orchestration, profiles, skills, memory policy, tool routing, safety policy, and sessions.
- The decided [[flynt-omegon-artifact-scope]] requires shared artifacts to be installed once and activated by scope; Flynt must not create a private duplicate Omegon universe.
- `flynt-agent` declares the storage rule: Flynt owns `.flynt/`; Omegon owns `.omegon/`.
- [[artifact-actions]] establishes the local layering precedent: `flynt-core` owns serializable action/request types; `flynt-app` owns navigation and project side effects.
- [[terminal-validation-actions]] establishes the ecosystem precedent: mature a capability locally behind separable contracts, reuse Omegon HostAction governance for agent-initiated host work, and extract a reusable Styrene substrate only after a second consumer proves the abstraction.
- [[apple-notes-import]] establishes the platform-provider precedent: the platform adapter owns supported OS interaction; normalization, planning, and final Flynt project mutation remain platform-neutral and testable.

## Ownership model

```text
Styrene ecosystem
  shared identity, reusable substrates, package and protocol conventions

Omegon
  agent orchestration, profiles, skills, memory, tool routing, safety,
  HostAction policy and session lifecycle

Flynt
  native workspace host, project data, typed project actions,
  navigation, capture import, OS registration and UI

flynt-agent
  project-scoped ACP adapter exposing Flynt semantics to Omegon

Recro / Scribe and other domain packages
  skills, playbooks, style, domain data and migration inputs
```

### Flynt owns

- registering and receiving Stable, Candidate, and Dev URL schemes;
- AppKit/UIKit lifecycle adapters and native presentation;
- typed Flynt project, document, task, view, and capture actions;
- resolving project identity and project-relative references;
- deterministic navigation and project mutation;
- capture normalization, destination policy, collision handling, asset placement, atomic import, indexing, sync notification, provenance, and user-visible receipts;
- Finder, Share Sheet, Shortcuts, and App Intent affordances whose semantics are deterministic Flynt operations.

### Omegon owns

- deciding and orchestrating agentic work;
- profiles, skills, package activation, model selection, memory, and session state;
- safety policy and approval contracts for agent-initiated native work;
- generic agent-facing capabilities that are not Flynt project semantics.

Omegon does not receive raw AppKit/UIKit lifecycle events, own Flynt window focus, define Flynt note/task URLs, or become a required dependency for deterministic capture and navigation.

### Styrene owns, when reuse is proven

- operator cryptographic identity and derived credentials;
- reusable cross-product substrates and contracts after at least two concrete hosts need them;
- package conventions and shared provider abstractions.

The first implementation remains in Flynt with clean extraction seams. Do not create a speculative `styrene-native` or generic invocation framework before a second product such as Auspex demonstrates compatible requirements.

### Domain packages own

Recro, Scribe, and similar packages own domain knowledge, playbooks, style, and migration data. They do not extend the permanent core URL vocabulary or native storage model. There are no core `flynt://recro/...` or `flynt://scribe/...` routes.

## Decisions

### Define a Flynt invocation contract, not a generic native ingress platform

**Status:** accepted

`flynt-core` owns a versioned, serializable action model such as `FlyntInvocation`, `FlyntLinkAction`, `CaptureEnvelope`, and `CaptureImportPlan`. Platform parsing and execution must be separable. `flynt-app` and `flynt-mobile` adapt OS events into those types and execute them through existing project/navigation command sources.

This is deliberately Flynt-domain-specific. A generic Styrene envelope may later be extracted from demonstrated commonality, not anticipated commonality.

### Use installation-specific schemes

**Status:** accepted

```text
flynt://             Stable
flynt-candidate://   Candidate
flynt-dev://         Dev
```

Stable alone may retain `flynt-note://` as a compatibility alias. Candidate and Dev must never contend with Stable for production links.

Initial route families are navigation and bounded capture only:

```text
flynt://project/<project-id>
flynt://note/<document-id>?project=<project-id>
flynt://task/<task-id>?project=<project-id>
flynt://view/<view-id>?project=<project-id>
flynt://capture?...bounded fields...
```

Durable links use project identity plus stable entity ID or project-relative reference. They never persist absolute machine paths, secrets, model prompts, shell commands, or credentials.

### Keep deterministic native actions independent of Omegon

**Status:** accepted

Operator-initiated Open, Reveal, Copy Link, Capture, Search, Create Task, and view-navigation actions execute in Flynt without starting an Omegon session. This preserves offline operation and the Flynt/Omegon deployment boundary.

Agent-initiated native operations must enter through Omegon's typed HostAction review and policy path, with Flynt acting as the executing/reviewing host when appropriate. They must not call AppKit, Finder, `open`, `osascript`, Shortcuts, or shell commands behind that boundary. Do not add a generic native HostAction catalog until a concrete second HostAction justifies extending the existing framework.

### Make native capture adapters thin

**Status:** accepted

Share extensions and other native providers extract supported OS values and stage a neutral, versioned capture envelope plus assets. They do not generate canonical Flynt Markdown/frontmatter, select final project paths, index content, or implement sync policy.

The main Flynt process owns validation and atomic promotion into a project. App Group storage is transient capture staging only; it contains no Omegon memory, model/provider credentials, Styrene root identity material, or durable project registry.

### Preserve shared identity and secret boundaries

**Status:** accepted

Flynt consumes `styrene-identity`; it does not introduce native-link or Share-extension identity stores. Provider/OAuth credentials remain in their established shared secret boundary. URLs and capture envelopes never carry secrets. A Share extension receives only the minimum App Group entitlement necessary for capture staging.

### Prefer lightweight Finder and Apple automation affordances

**Status:** accepted

First-class actions are:

- Open Folder or Markdown File in Flynt;
- Reveal Active Note in Finder;
- Copy Flynt Link;
- Save selection, URL, or files to the Flynt inbox;
- App Intents/Shortcuts for deterministic Flynt project actions.

Use the shared typed invocation/capture contracts rather than independent Finder logic. Defer Finder Sync because Flynt does not own a provider sync root requiring badges. Reject File Provider unless the product explicitly adopts a virtual-filesystem responsibility.

### Keep Flynt App Intents distinct from Omegon intents

**Status:** accepted

Flynt App Intents expose deterministic project actions: capture, open, search, create task, and navigate. Generic Ask Agent, Run Prompt, Resume Agent, or Run Playbook intents belong to Omegon. Any future Flynt-specific agent intent must connect through a verified project-scoped Omegon deployment and its permission policy.

### Preserve filesystem ownership

**Status:** accepted

Native invocation and capture may write Flynt-owned project content and `.flynt/` runtime state through existing APIs. They do not mutate `.omegon/`. Omegon may access these actions only through `flynt-agent` or an approved HostAction boundary.

## Rejected approaches

- A Flynt-owned generic automation runtime parallel to Omegon.
- A speculative shared Styrene native-integration crate before a second consumer exists.
- Native extensions that directly write canonical Flynt Markdown/frontmatter.
- Finder extensions that invent a second project registry or claim provider sync status.
- Finder Sync solely for contextual actions available through Services, Share, or Shortcuts.
- File Provider while Flynt remains an ordinary-files local-first application.
- Generic prompt/agent execution encoded in `flynt://` URLs.
- Destructive routes such as delete, shell, sync-push, approve, or agent-run.
- Core native routes tied to Recro, Scribe, or another package.
- New identity, credential, or memory storage in App Group or `.flynt/` for concerns already owned by Styrene/Omegon.

## Security and portability invariants

- Treat every OS URL, shared item, and file reference as hostile input.
- Decode once; reject malformed encoding, NUL, absolute paths where relative paths are expected, and all `..` traversal.
- Resolve project-relative paths against the canonical selected project root and verify containment after canonicalization.
- Unknown projects require explicit resolution; never silently substitute the last-opened project.
- Navigation actions are read-only. Capture is the only initial write-capable URL family and has bounded payload size and accepted media types.
- No route executes shell commands, invokes an agent, approves a HostAction, or transports credentials.
- Import uses staging, collision-safe names, atomic promotion, idempotency IDs, and recoverable failure quarantine.
- Plaintext project portability remains authoritative; native integrations are adapters, not required storage formats.

## Existing implementation debt to resolve first

The iOS Share Extension is implemented but its producer and consumer disagree:

- Swift currently writes `codex-inbox`.
- Rust drains `flynt-inbox`.
- Bundle IDs and App Group identifiers mix `io.styrene.codex` and `io.styrene.flynt` across Dioxus metadata, plists, entitlements, Just recipes, and release workflows.

Continuity may require retaining existing production identifiers. The resolution must therefore be an explicit compatibility matrix and validator, not a blind rename. The Share Extension's concurrent `NSItemProvider` callbacks must also serialize result collection. Existing App Group captures must remain readable through any migration.

## Implementation sequence

### Phase 1 — Invocation and capture core

1. Add typed invocation/link and capture envelope models to `flynt-core`.
2. Add strict parser, serializer, compatibility alias, size limits, and traversal tests.
3. Add project/entity resolution against existing Flynt command and registry boundaries.
4. Add an atomic capture importer with receipts, quarantine, collision tests, and no `.omegon/` writes.

### Phase 2 — Apple lifecycle adapters

1. Register identity-specific schemes in Stable/Candidate/Dev bundles.
2. Receive cold-start and already-running URLs on macOS and iOS.
3. Execute through shared Flynt route/command sources rather than surface-specific shortcuts.
4. Make malformed and unresolved links visible without implicit writes.

### Phase 3 — Repair Share to Flynt

1. Define and validate the production bundle/App Group/inbox compatibility matrix.
2. Convert the Swift extension from Markdown writer to neutral envelope producer.
3. Serialize provider extraction and report unsupported items.
4. Drain envelopes transactionally in the main app.
5. Add end-to-end tests for URL, text, image, collision, retry, partial failure, and legacy inbox migration.

### Phase 4 — Native affordances

1. macOS Share/Services or Shortcut-based capture.
2. Reveal in Finder, Open in Flynt, and Copy Flynt Link.
3. Flynt App Intents for deterministic open/search/capture/task operations.
4. Consider Quick Look for Flynt-specific artifact formats after invocation and artifact resolution are stable.

### Explicit deferrals

- shared Styrene invocation/native crate;
- generic native HostAction family;
- Finder Sync;
- File Provider;
- generic Omegon App Intents;
- domain-package URL namespaces;
- agent-triggered native actions without a concrete reviewed HostAction design.

## Acceptance criteria

- Stable, Candidate, and Dev route only their own URL schemes.
- Cold-start and already-running delivery produce identical typed actions.
- A project link contains no absolute local path and resolves after project relocation.
- Traversal, malformed encoding, unknown project, and oversized capture inputs fail visibly and without mutation.
- Deterministic capture/navigation works with Omegon unavailable and leaves `.omegon/` untouched.
- Agent-originated native work cannot bypass Omegon HostAction review.
- A Safari Share creates exactly one indexed Flynt inbox item; retry does not duplicate it.
- An image capture creates one document and reachable project-contained assets or remains wholly recoverable in quarantine.
- Existing TestFlight App Group captures survive identifier/inbox compatibility repair.
- Recro, Scribe, and other package concepts do not enter the core invocation schema.
- A repository validator detects drift among plist, entitlements, Dioxus metadata, Just recipes, Swift constants, Rust consumers, and release workflow identifiers.

## Assumptions reviewed

The design previously risked leaving these assumptions implicit. They are now resolved as follows:

- **[resolved assumption]** Native OS reception belongs to the active app host, not Omegon.
- **[resolved assumption]** Flynt project semantics are specific enough to remain in Flynt before shared extraction.
- **[resolved assumption]** Deterministic native actions must work without model/runtime availability.
- **[resolved assumption]** Existing iOS identifiers may be continuity constraints; compatibility evidence is required before renaming.
- **[resolved assumption]** Finder Sync is unnecessary until Flynt owns a sync-root badge requirement.
- **[resolved assumption]** App Group storage is transient ingress, not a general cross-product state channel.
- **[resolved assumption]** Agent-initiated native mutation requires the existing Omegon-governed HostAction path.

## What assumptions is this design making that have not been stated?

None remain open at decision time. New platform constraints discovered during implementation must be added as explicit questions and move this node back to `exploring` before changing an ownership boundary.
