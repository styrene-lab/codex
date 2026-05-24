+++
id = "flynt-0-11-0-milestone"
kind = "design_node"
title = "Flynt 0.11.0 milestone — research workspace and Omegon 0.23 ACP integration"
status = "active"
tags = ["release", "milestone", "flynt-0.11.0", "research", "omegon", "acp"]

[data]
issue_type = "milestone"
release_target = "0.11.0"
base_release = "0.10.8"
priority = 1
+++

# Flynt 0.11.0 milestone

## Milestone

**Name:** Flynt 0.11.0 — Research workspace foundation and Omegon 0.23 ACP integration

**Base build:** 0.10.8

**Target release:** 0.11.0

**Goal:** ship the next minor Flynt release with the source-backed research workspace groundwork already committed after 0.10.8, plus compatibility with Omegon 0.23.x and its ACP additions.

This is a minor release because it combines new user-facing/product-direction work (`feat(research): add source note groundwork`) with an agent protocol integration update. It should not be cut as 0.10.9 unless the Omegon 0.23 work is explicitly deferred.

## Current delta from 0.10.8

Commits currently queued locally after `v0.10.8`:

| Commit | Scope | Release note |
| --- | --- | --- |
| `b4b7213` | Research/source note groundwork | Starts the source-backed research workspace foundation in Flynt's agent and core templates. |
| `7454793` | Source task canvas projections | Defines how captured sources project into task/canvas workflows. |
| `fd11c89` | Eidolon embedded viewer integration | Lays out the embedded viewer integration path for research artifacts and source review. |
| `bd20614` | Portable analysis bundles | Defines portable bundles for source artifacts, provenance, manifests, and analysis outputs. |

Changed files in the queued delta:

- `crates/flynt-agent/src/extension.rs`
- `crates/flynt-core/src/templates.rs`
- `design/source-task-canvas-projections.md`
- `design/typed-source-relationships.md`
- `design/zotero-research-workspace.md`
- `docs/eidolon-embedded-viewer-integration.md`
- `docs/portable-analysis-bundles.md`

## Release criteria

Flynt 0.11.0 is releasable when all of the following are true:

- The source-note groundwork committed after 0.10.8 is validated and documented in the changelog.
- Research workspace design docs describe Flynt as a source-backed research workspace, not a Zotero clone.
- Portable analysis bundles preserve provenance, source manifests, access scope, authorization notes, and artifacts.
- Eidolon embedded viewer integration has a documented boundary for viewing captured/source-backed evidence inside Flynt.
- Source task/canvas projections have a documented data-flow and do not duplicate source truth outside the artifact system.
- Flynt integrates with Omegon 0.23.x ACP additions.
- Flynt remains compatible with the 0.10.8 release pipeline: the tag version must match `Cargo.toml` workspace version.
- `Cargo.toml` and `CHANGELOG.md` are bumped to `0.11.0` before tagging `v0.11.0`.
- Rust validation passes for the touched crates/workspace.

## Workstreams

| Workstream | Feature ID | Priority | Depends on | Outcome |
| --- | --- | --- | --- | --- |
| Release packaging | F11-01 | P0 | existing release workflow | Version/changelog/tag path for 0.11.0 is clean. |
| Source note groundwork | F11-02 | P0 | queued commit `b4b7213` | Flynt includes the first source-note templates/agent extension groundwork. |
| Research workspace docs | F11-03 | P0 | queued design/docs commits | The research direction is documented as source-backed, artifact-native, and provenance-aware. |
| Portable analysis bundles | F11-04 | P0 | artifact system | Analysis bundles are specified with provenance and source manifests. |
| Eidolon embedded viewer | F11-05 | P1 | Eidolon viewer assumptions | Flynt has an integration design for embedded source/artifact viewing. |
| Source task/canvas projections | F11-06 | P1 | canvas/task surfaces | Captured sources can be represented in task/canvas workflows without duplicating source truth. |
| Omegon 0.23 ACP integration | F11-07 | P0 | Omegon 0.23.x ACP additions; GitHub issue #21 | Flynt handles Omegon 0.23 ACP additions for structured plan state and profile defaults. |
| Storage policy first pass | F11-08 | P0 | dogfooding-generated state sprawl | Opening folders starts moving toward external local runtime state instead of implicit in-project `.flynt-local/` creation. |

## F11-01: Release packaging

### Implementation tasks

| Task ID | Scope | Files likely touched | Acceptance |
| --- | --- | --- | --- |
| F11-01.1 | Version bump | `Cargo.toml`, `Cargo.lock` if needed | Workspace package version is `0.11.0`. |
| F11-01.2 | Changelog | `CHANGELOG.md` | `0.11.0` entry summarizes research workspace groundwork and Omegon 0.23 ACP integration. |
| F11-01.3 | Validation | workspace | `cargo check`, targeted tests, and release-relevant validation pass. |
| F11-01.4 | Release tag | git | `v0.11.0` is created only after the version bump commit lands. |

## F11-02: Source note groundwork

### Implementation tasks

| Task ID | Scope | Files likely touched | Acceptance |
| --- | --- | --- | --- |
| F11-02.1 | Agent extension surface | `crates/flynt-agent/src/extension.rs` | New source-note capability is visible through the Flynt agent extension surface. |
| F11-02.2 | Core templates | `crates/flynt-core/src/templates.rs` | Source note templates render consistently with existing Flynt templates. |
| F11-02.3 | Tests or smoke validation | touched crates | Template/extension changes are covered by unit tests or a documented smoke check. |

## F11-03: Research workspace docs

### Implementation tasks

| Task ID | Scope | Files likely touched | Acceptance |
| --- | --- | --- | --- |
| F11-03.1 | Research positioning | `design/zotero-research-workspace.md` | The design states Flynt is source-backed research workspace tooling rather than a Zotero clone. |
| F11-03.2 | Typed source relationships | `design/typed-source-relationships.md` | Source relationships are typed and artifact-backed. |
| F11-03.3 | Release note summary | `CHANGELOG.md` | The shipped release note explains the direction without overclaiming unimplemented capture/browser automation. |

## F11-04: Portable analysis bundles

### Implementation tasks

| Task ID | Scope | Files likely touched | Acceptance |
| --- | --- | --- | --- |
| F11-04.1 | Bundle contract | `docs/portable-analysis-bundles.md` | Bundle contents include provenance, source manifest, access scope, authorization notes, artifacts, and analysis outputs. |
| F11-04.2 | Artifact boundary | docs/design | The bundle spec builds on Flynt's existing artifact system rather than creating a parallel storage layer. |
| F11-04.3 | Safety boundary | docs/design | The spec supports authorized evidence capture/provenance and does not frame stealth or evasion as a feature. |

## F11-05: Eidolon embedded viewer integration

### Implementation tasks

| Task ID | Scope | Files likely touched | Acceptance |
| --- | --- | --- | --- |
| F11-05.1 | Viewer integration design | `docs/eidolon-embedded-viewer-integration.md` | The doc defines responsibilities between Flynt, artifacts, and Eidolon. |
| F11-05.2 | Source review flow | docs/design | The intended review flow for captured/source-backed evidence is explicit. |
| F11-05.3 | Deferral boundary | docs/design | Any unimplemented viewer work is clearly marked as design/future work. |

## F11-06: Source task/canvas projections

### Implementation tasks

| Task ID | Scope | Files likely touched | Acceptance |
| --- | --- | --- | --- |
| F11-06.1 | Projection model | `design/source-task-canvas-projections.md` | The doc explains how source-backed work appears in task and canvas surfaces. |
| F11-06.2 | Source truth boundary | design docs | Projections reference source artifacts and do not become independent truth copies. |
| F11-06.3 | Release scope | `CHANGELOG.md` | The release note distinguishes shipped groundwork from future UI implementation. |

## F11-07: Omegon 0.23 ACP integration

### Context

The known tracking item is GitHub issue #21 for Omegon 0.23 integration updates around structured plan state and profile defaults.

### Implementation tasks

| Task ID | Scope | Files likely touched | Acceptance |
| --- | --- | --- | --- |
| F11-07.1 | ACP compatibility audit | `crates/flynt-agent`, `crates/flynt-app` agent rail/session code | Flynt's embedded Omegon integration is checked against Omegon 0.23.x ACP additions. |
| F11-07.2 | Structured plan state | agent rail / ACP event handling | Flynt renders or safely ignores structured plan-state events without breaking chat streaming. |
| F11-07.3 | Profile defaults | agent launch/profile configuration | Flynt uses Omegon 0.23 profile defaults instead of hardcoded ACP defaults where possible. |
| F11-07.4 | Backward compatibility | ACP launch/session tests | Existing 0.10.8 behavior still works for older/default ACP surfaces, or the minimum Omegon version is documented. |
| F11-07.5 | Issue linkage | GitHub issue #21, changelog | Issue #21 is updated/closed with the implementation commit references, and `CHANGELOG.md` calls out Omegon 0.23.x support. |

## F11-08: Storage policy first pass

### Context

Dogfooding Flynt in its own repository exposed a general product issue: opening a folder can create multiple Flynt-owned directories in the content root, making portable metadata and local runtime state look like competing project truth. The policy is documented in [Flynt storage policy](flynt-storage-policy.md).

### Implementation tasks

| Task ID | Scope | Files likely touched | Acceptance |
| --- | --- | --- | --- |
| F11-08.1 | Storage policy design | `design/flynt-storage-policy.md` | Policy distinguishes content root, portable metadata, local runtime state, and canonical truth without special-casing this repo. |
| F11-08.2 | Index DB default | `crates/flynt-store/src/project.rs` | New default index DB path resolves outside the content root. |
| F11-08.3 | Compatibility | `crates/flynt-store/src/project.rs` | Explicit absolute `flynt_index_db_path` and `local_state_root` overrides still work. |
| F11-08.4 | Track index snapshot | `crates/flynt-core/src/models.rs`, `crates/flynt-app/src/views/settings.rs`, `crates/flynt-store/src/project.rs` | Settings expose an opt-in deterministic JSONL metadata snapshot while SQLite remains local runtime state. |
| F11-08.5 | Follow-up boundary | milestone/changelog | Remaining `.flynt-local` writers are called out as follow-up work, not silently treated as solved. |

## Non-goals for 0.11.0

- Full Zotero replacement behavior.
- Stealth browser automation or evasion-oriented capture workflows.
- Full Eidolon viewer implementation if only the integration contract is ready.
- A complete visual source-task canvas UI if only the projection model is ready.
- Crates.io publication through `release-plz`; `release-plz` remains dormant unless separately enabled.

## Release checklist

- [ ] Verify local queued commits are pushed to the release branch.
- [ ] Complete F11-07 Omegon 0.23 ACP integration.
- [ ] Run targeted Rust validation for touched crates.
- [ ] Update `Cargo.toml` to `0.11.0`.
- [ ] Update `CHANGELOG.md` with a `0.11.0` entry.
- [ ] Commit release bump with `chore: bump version to 0.11.0`.
- [ ] Tag `v0.11.0` only after the version in `Cargo.toml` matches the tag.
- [ ] Push branch and tag to trigger `.github/workflows/release.yml`.
