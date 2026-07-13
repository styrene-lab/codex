---
id: apple-notes-import
title: "Apple Notes import for Flynt 0.13.0"
status: exploring
priority: 1
parent: happy-install-sync-ux
tags: [onboarding, migration, macos, apple-notes, privacy, 0.13.0]
open_questions:
  - "[assumption] Notes.app's Apple Events dictionary remains available and materially compatible on supported macOS 13+ releases."
  - "[assumption] A Developer ID build with hardened runtime can request Notes Automation access using NSAppleEventsUsageDescription and com.apple.security.automation.apple-events without Full Disk Access."
  - "[assumption] Stable Notes identifiers are stable enough to deduplicate user-initiated repeat imports on the same Apple account."
  - "[assumption] Attachment save behavior is reliable enough to support bounded per-note export without reading Apple's private Notes database."
  - "Which Notes HTML constructs and attachment classes must be represented as explicit degradation warnings for the 0.13.0 fixture corpus?"
  - "Should a repeated import update the existing Flynt document automatically, or require a per-note replace/keep-both decision?"
dependencies: [local-first-onboarding-baseline]
related: [happy-install-sync-ux]
---

# Apple Notes import for Flynt 0.13.0

## Overview

Offer a macOS-only, user-initiated path to copy substantial material from Apple Notes into a Flynt project while leaving Apple Notes unchanged. The feature serves both first-run migration and later selective imports. It must use Apple-supported Automation/Apple Events rather than private Notes databases or opaque iCloud paths.

## Product contract

- **Copy only:** Flynt never edits, deletes, archives, or reorganizes Apple Notes.
- **Explicit access:** macOS Automation permission is requested only after the operator selects Apple Notes import.
- **Review before copy:** account, folder, and note metadata can be inspected before full bodies and attachments are exported.
- **Selective by default:** operators may choose folders or individual notes; importing every account is not the default.
- **Local processing:** note titles, bodies, and attachments are not sent to Omegon or a model and are not written to logs.
- **Stable provenance:** imported documents record Apple Notes identity, account/folder context, source timestamps, import version, and warnings.
- **Honest fidelity:** locked notes and unsupported rich objects are skipped or reported, never silently discarded.
- **No raw platform paths:** UI presents account/folder/note identities and Flynt project names, not Apple-internal filesystem locations.

## Verified macOS capability

The installed Notes scripting dictionary exposes accounts, nested folders, notes, and attachments. Notes expose name, stable identifier, containing folder, HTML body, plaintext body, creation/modification dates, password-protected state, shared state, and attachments. Attachments expose name, identifier, content identifier, dates, URL, and a save command.

This is sufficient for a supported migration path without reading `NoteStore.sqlite`.

## 0.13.0 MVP boundary

### Include

1. macOS availability and Automation-permission diagnostics.
2. Structured account/folder/note-summary discovery.
3. First-run and existing-project entry points.
4. Folder and individual-note selection.
5. Selected-note body export from Notes HTML to Markdown.
6. Supported attachment export into a project-contained asset directory.
7. Locked-note exclusion and shared-note warning.
8. Stable-ID deduplication and a user-visible import report.
9. Progress, cancellation, and bounded subprocess timeouts.
10. Provenance metadata and copy-only language throughout the UX.

### Defer

- background or bidirectional synchronization;
- source deletion or archival;
- smart-folder recreation;
- collaboration semantics;
- iOS import;
- guaranteed lossless handwriting, scans, drawings, or Notes-only rich objects;
- direct access to Apple private databases.

## Architecture

```text
Notes.app / Apple Events
        |
        v
AppleNotesProvider (macOS-only)
  - availability
  - discover summaries
  - export selected records
        |
        v
AppleNotesExportRecord
        |
        +--> HTML -> Markdown normalization
        +--> attachment staging
        +--> fidelity warnings
        |
        v
Import planner
  - destination paths
  - stable-ID duplicate decisions
  - provenance metadata
        |
        v
Flynt project staging and atomic promotion
```

The provider owns platform interaction. Normalization, duplicate planning, and report generation remain platform-neutral and testable with fixtures.

## Provider execution contract

The first implementation uses `/usr/bin/osascript` with a fixed bundled JXA/AppleScript program and structured JSON output. User-controlled account, folder, and note values must be passed as process arguments or filtered after discovery; they must never be interpolated into script source. Child processes use piped stdio, bounded timeouts, and explicit cancellation.

Discovery fetches metadata only. Full HTML and attachment work occurs only for selected notes to avoid Apple Event amplification on large libraries.

## Permission and failure states

- `unsupported_platform`: Apple Notes import is macOS-only.
- `notes_unavailable`: Notes.app or `osascript` is unavailable.
- `permission_required`: discovery has not yet been authorized.
- `permission_denied`: macOS denied Automation access; UI explains System Settings → Privacy & Security → Automation.
- `notes_locked`: selected note is password protected and is skipped.
- `notes_timeout`: Notes did not answer within the bounded operation.
- `malformed_response`: provider output failed schema validation.
- `partial_export`: readable note imported with explicit fidelity warnings.

No failure message includes note bodies or private attachment paths.

## Imported document provenance

```toml
source_format = "apple_notes"
source_path = "apple-notes://<stable-id>"
imported_reference = true

[metadata]
apple_notes_note_id = "<stable-id>"
apple_notes_account = "iCloud"
apple_notes_folder = "Work/Research"
apple_notes_created_at = "..."
apple_notes_modified_at = "..."
apple_notes_shared = false
apple_notes_import_version = 1
```

Repeated imports use `apple_notes_note_id` as identity. Filename collisions alone never establish identity.

## First implementation slice

1. Add the macOS provider model and fixed metadata-discovery script.
2. Parse and validate structured summaries without logging content.
3. Classify unavailable, denied, timeout, process, and malformed-response errors.
4. Add provider fixture tests and a macOS ignored smoke test.
5. Add signing metadata required for Automation access.

This slice deliberately stops before mutating a project. It proves the permission and discovery boundary before body conversion and attachment handling are allowed to write user data.

## Open Questions

- [assumption] Notes.app's Apple Events dictionary remains available and materially compatible on supported macOS 13+ releases.
- [assumption] A Developer ID build with hardened runtime can request Notes Automation access using `NSAppleEventsUsageDescription` and `com.apple.security.automation.apple-events` without Full Disk Access.
- [assumption] Stable Notes identifiers are stable enough to deduplicate user-initiated repeat imports on the same Apple account.
- [assumption] Attachment save behavior is reliable enough to support bounded per-note export without reading Apple's private Notes database.
- Which Notes HTML constructs and attachment classes must be represented as explicit degradation warnings for the 0.13.0 fixture corpus?
- Should a repeated import update the existing Flynt document automatically, or require a per-note replace/keep-both decision?
