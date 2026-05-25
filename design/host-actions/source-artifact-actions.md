+++
id = "source-artifact-actions"
kind = "design_node"
title = "Source artifact HostActions"
status = "exploring"
tags = ["host-actions", "research", "sources", "eidolon", "bundles"]

[data]
issue_type = "feature"
priority = 1
parent = "flynt-host-actions-platform"
+++

# Source artifact HostActions

## Overview

HostActions can turn research/source tool results into direct workspace affordances: open captured evidence, inspect bundles, export portable analysis, or route an artifact into a viewer.

## Candidate action families

- `flynt.source.open@1`
- `flynt.bundle.open@1`
- `flynt.bundle.export@1`
- `flynt.eidolon.open@1`

## Use cases

- A capture tool returns “12 artifacts collected” plus an action to open the bundle.
- A research agent cites a source and provides a one-click action to view the source artifact.
- A provenance checker returns an action to export a portable analysis bundle.
- A PDF/web/archive artifact opens in Eidolon or an embedded viewer rather than plain text chat.

## Open questions

- [assumption] Source artifacts and bundles have stable IDs/paths in Flynt's artifact model.
- What viewer routing belongs in Flynt versus Eidolon?
- Should source open actions require access-scope/authorization metadata in params?
- Can bundle export actions be made deterministic and reviewable before writing files?
- How do we prevent HostActions from becoming a stealth browser automation path?

## Safety constraints

- Source actions open or export already-authorized artifacts; they do not evade access controls.
- Viewer actions must preserve provenance and access-scope notes.
- Export actions should show a manifest preview before writing portable bundles.
