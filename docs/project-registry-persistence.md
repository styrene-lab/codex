---
id: project-registry-persistence
title: "Project registry snapshot persistence"
status: decided
tags: [project-registry, persistence, sync, plaintext-portability]
open_questions: []
dependencies: []
related: []
---

# Project registry snapshot persistence

## Overview

Persist a generated, deterministic, disposable ProjectRegistrySnapshot at .flynt/registry/project-registry.snapshot.json. The snapshot is a portable derived index for startup diagnostics, sync visibility, and graph inspection; plaintext project files and existing stores remain authoritative. The persisted snapshot must never contain absolute paths, raw document content, or proprietary binary payloads.

## Decisions

### Persist snapshots as generated derived state

**Status:** accepted

**Rationale:** The registry snapshot helps startup diagnostics and sync visibility, but making it authoritative would compromise the Obsidian-compatible plaintext project model. The snapshot must be safe to delete and rebuild from source files and stores.

### Use .flynt/registry/project-registry.snapshot.json

**Status:** accepted

**Rationale:** A JSON file under .flynt/registry is explicit, inspectable, portable, and separated from user-authored notes. The filename makes the generated/snapshot nature visible.
