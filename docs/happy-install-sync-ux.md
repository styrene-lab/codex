---
id: happy-install-sync-ux
title: "Happy install, first-run, and project sync UX"
status: exploring
tags: [onboarding, installation, icloud, git-sync, ux, local-first]
open_questions:
  - "[assumption] A fresh macOS install can create and open a useful local project without requiring a pre-existing directory, Finder interaction, or sync selection."
  - "[assumption] iCloud Drive capability and project residency can be detected reliably enough to disable or explain unavailable actions before mutation."
  - "[assumption] Existing iCloud project migration preserves the source until copy verification and successful destination reopen complete."
  - "[assumption] Current sync status projection distinguishes local save, iCloud availability, Git commit, Git push, authentication failure, and conflict states."
  - "Which smallest first implementation slice guarantees fresh launch → editable note → clear 'Saved on this Mac' state without forcing sync?"
  - "What assumptions is this design making that haven't been stated?"
dependencies: []
related: []
---

# Happy install, first-run, and project sync UX

## Overview

Create a zero-technical-savvy happy path from installation and first launch through immediate local editing, with optional iCloud syncing as the recommended Apple path and progressive Git backup for advanced/team users. Success means users can understand whether their work is safe without Terminal, filesystem paths, or Git vocabulary.

## Open Questions

- [assumption] A fresh macOS install can create and open a useful local project without requiring a pre-existing directory, Finder interaction, or sync selection.
- [assumption] iCloud Drive capability and project residency can be detected reliably enough to disable or explain unavailable actions before mutation.
- [assumption] Existing iCloud project migration preserves the source until copy verification and successful destination reopen complete.
- [assumption] Current sync status projection distinguishes local save, iCloud availability, Git commit, Git push, authentication failure, and conflict states.
- Which smallest first implementation slice guarantees fresh launch → editable note → clear 'Saved on this Mac' state without forcing sync?
- What assumptions is this design making that haven't been stated?
