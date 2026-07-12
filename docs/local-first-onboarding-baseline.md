---
id: local-first-onboarding-baseline
title: "Local-first onboarding baseline"
status: exploring
parent: happy-install-sync-ux
tags: [onboarding, local-first, first-run, ux]
open_questions:
  - "Should the default first project remain `~/Documents/Flynt`, or be a named child such as `~/Documents/Flynt/My Project`? Recommendation: use a named child to avoid treating the project collection directory as a project."
  - "What starter document and copy should open after setup? Recommendation: `Welcome.md` with a short editable heading and three plain-language next steps, not demo-heavy content."
  - "Where should the initial `Saved on this Mac` safety indicator live? Recommendation: global toolbar/project chrome, not a task-level pill."
  - "[assumption] Initializing a local Git repository during zero-savvy local setup is invisible and harmless. Verify whether it causes prompts, errors, or misleading backup expectations."
  - "[assumption] The default Documents directory is writable and appropriate; setup needs an actionable fallback when it is not."
  - "What assumptions is this design making that haven't been stated?"
dependencies: []
related: []
---

# Local-first onboarding baseline

## Overview

Implement the first milestone of the happy install and sync target: from fresh launch, one primary action creates a useful local project, persists it in launcher state, opens an editable starter document, and communicates that work is saved locally. This establishes the safe baseline before iCloud migration or Git backup is offered.

## Research

### Current first-run route and primary action

App derives `show_welcome` from `launcher_profile.completed_onboarding`; when true it renders `WelcomeView`. The current primary `on_get_started` handler merely marks onboarding complete and keeps the already-open runtime root. Evidence: `crates/flynt-app/src/app.rs:921-978`. `WelcomeView` renders the primary label `Start with this local project` and presents open/import/clone/cloud as peer cards. Evidence: `crates/flynt-app/src/views/welcome.rs:5-104`.

### Default runtime root and project initialization

Bootstrap selects the first launcher-profile project, else legacy root, else `~/Documents/Flynt`; `runtime_state_for_project_root` creates the directory, opens `Project`, starts watchers/sync, and seeds demo publication repo files only. It does not establish a named first project or open a starter note. Evidence: `crates/flynt-app/src/bootstrap.rs:1241-1340,1417-1457`.

### Existing create/switch primitives

`PendingProjectSetup::Create` is handled by `OmegonRuntimeContext::execute_project_setup`, which creates the directory, initializes Git, opens Project, and persists it to launcher profile. The app then calls `switch_project_runtime`, clears tabs, and routes to Notes. Evidence: `crates/flynt-app/src/bootstrap.rs:280-360`; `crates/flynt-app/src/app.rs:590-690`; project selector uses the same flow in `crates/flynt-app/src/components/sidebar.rs:1190-1277`.

### Starter-document primitives

Project can save/index canonical Markdown through `save_document_content`; document tabs open with `TabState::open(DocumentId, title)`. Existing app creation flows already create artifacts and immediately open returned document ids, so the local-first setup should use the same store and tab-state interfaces rather than filesystem-only writes. Evidence: `crates/flynt-store/src/project.rs`; `crates/flynt-app/src/state.rs`; design creation flows under `crates/flynt-app/src/components/design_panel.rs`.

### Project safety status gap

Toolbar auto-sync labels expose Git mechanics (`Idle`, `Committing`, `Pulling`, `Pushing`), while `SyncStatusPill::LocalOnly` describes task-to-upstream issue linkage. Neither is an appropriate first-run project safety signal. The first milestone needs a separate projection that can truthfully say `Saved on this Mac` independently of forge/task sync. Evidence: `crates/flynt-app/src/components/toolbar.rs:103-217`; `crates/flynt-app/src/components/sync_status_pill.rs:252-305`.

## Decisions

### Use one primary local-start action

**Status:** accepted

**Rationale:** Fresh users need one obvious path. Existing open/import/Git/cloud options remain available but are visually secondary and must not block starting.

### Reuse existing project setup and runtime switching

**Status:** accepted

**Rationale:** PendingProjectSetup, launcher-profile persistence, and switch_project_runtime already encode lifecycle ownership. A parallel first-run initializer would create drift and recovery inconsistencies.

### Create starter content through Project APIs

**Status:** accepted

**Rationale:** Using canonical save/index APIs guarantees identity, indexing, watchers, and immediate tab opening; raw filesystem-only seeding would bypass those contracts.

### Model project safety separately from issue sync

**Status:** accepted

**Rationale:** Task-to-forge sync and project storage safety are different systems. Reusing LocalOnly or upstream-issue labels would mislead nontechnical users.

### Keep advanced entry points available but secondary

**Status:** accepted

**Rationale:** Existing users still need open, import, clone, and cloud creation. Hiding them entirely harms recovery; presenting them as peer choices harms first-run comprehension.

## Open Questions

- Should the default first project remain `~/Documents/Flynt`, or be a named child such as `~/Documents/Flynt/My Project`? Recommendation: use a named child to avoid treating the project collection directory as a project.
- What starter document and copy should open after setup? Recommendation: `Welcome.md` with a short editable heading and three plain-language next steps, not demo-heavy content.
- Where should the initial `Saved on this Mac` safety indicator live? Recommendation: global toolbar/project chrome, not a task-level pill.
- [assumption] Initializing a local Git repository during zero-savvy local setup is invisible and harmless. Verify whether it causes prompts, errors, or misleading backup expectations.
- [assumption] The default Documents directory is writable and appropriate; setup needs an actionable fallback when it is not.
- What assumptions is this design making that haven't been stated?

## Implementation Notes

### File Scope

- `crates/flynt-app/src/views/welcome.rs` — 
- `crates/flynt-app/src/app.rs` — 
- `crates/flynt-app/src/bootstrap.rs` — 
- `crates/flynt-app/src/components/toolbar.rs` — 
- `crates/flynt-app/src/state.rs` — 
- `crates/flynt-store/src/project.rs` — 
- `crates/flynt-app/assets/main.css` — 

### Constraints

- Do not mark onboarding complete until project creation, starter-document creation, launcher-profile persistence, runtime switch, and document open all succeed.
- Do not delete or overwrite an existing path; derive a collision-free local project directory or ask for a decision.
- Do not use task SyncStatus to represent project storage safety.
- Keep open/import/clone/cloud paths operational and covered by existing behavior.
- Every failure must leave a valid runtime project and welcome/recovery surface.
