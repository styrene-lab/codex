---
id: self-update-servicing
title: "Self-update process servicing"
status: exploring
tags: [updates, settings, release, packaging]
open_questions:
  - "[assumption] The launcher profile is the correct durable storage location for Flynt update channel pinning because `OmegonRuntimeContext::load_launcher_profile().flynt_update_channel` already drives update checks."
  - "How should Stable compatible-line detection behave before 1.0: patch-only within current minor (0.12.x), or broader semver-compatible updates if explicitly opted in?"
  - "What install actions should be exposed per install source: direct-download PKG/DMG download+open, Homebrew command copy, Nix command copy, development build no-op, unknown source fallback?"
  - "Should operators be able to skip/snooze a specific stable patch version, and where should that skip state be stored?"
dependencies: []
related: []
---

# Self-update process servicing

## Overview

Service Flynt's internal update process so operators can update from inside the app instead of returning to GitHub Releases. Settings must expose a clear Updates surface with channel pinning: Stable tracks the current compatible release line (e.g. 0.12.x) and Nightly tracks signed nightly builds. Major-version updates are deferred to manual update flow because pre-1.0 majors may involve breaking changes.

## Research

### Current update implementation

Current code evidence: `crates/flynt-app/src/self_update.rs` already has stable/nightly channels, GitHub release fetch, signed `flynt-release.json` manifest verification, direct macOS artifact selection, install-source detection, and `configured_channel()` reading `flynt_update_channel` from launcher profile. `crates/flynt-app/src/components/toolbar.rs` currently performs toolbar update checks and presents update actions. `crates/flynt-app/src/views/settings.rs` already references `UpdateChannel` and should be inspected as the likely settings integration point.

### Toolbar badge adversarial assessment

Assessment found two issues: (1) after Stable was made compatible-line-only, a newer incompatible release such as 0.13.0 would have been reported as Current/no update, hiding the manual update path; (2) toolbar dismiss saved skipped_update_version but did not update local UI state immediately, so the pill could remain until remount. Implementation added UpdateState::ManualRequired and a local skipped_update signal in toolbar.

## Decisions

### Stable channel tracks only the current compatible release line

**Status:** proposed

**Rationale:** For 0.x releases, a minor version jump can carry breaking behavior. Stable should offer patch updates within the current line, such as 0.12.x, and defer a 0.13.0-style update to a manual release-page flow with explicit operator review.

### Nightly channel requires signed manifest verification

**Status:** proposed

**Rationale:** Nightly builds are higher churn and should never be selected from unsigned GitHub metadata alone. Existing code already requires signed manifests for nightly; servicing should preserve and surface that contract.

### Settings owns durable update channel pinning

**Status:** proposed

**Rationale:** The operator needs an explicit durable place to choose Stable or Nightly. The toolbar may advertise update availability, but Settings should own channel choice, verification details, last check status, and install-source-specific guidance.

### Stable line jumps surface as manual-required updates

**Status:** accepted

**Rationale:** Adversarial review found that treating incompatible newer Stable releases as Current would silently hide important updates. Stable patch-line updates remain automatic candidates; newer minor/major lines surface as manual-required updates that route to the release page.

## Open Questions

- [assumption] The launcher profile is the correct durable storage location for Flynt update channel pinning because `OmegonRuntimeContext::load_launcher_profile().flynt_update_channel` already drives update checks.
- How should Stable compatible-line detection behave before 1.0: patch-only within current minor (0.12.x), or broader semver-compatible updates if explicitly opted in?
- What install actions should be exposed per install source: direct-download PKG/DMG download+open, Homebrew command copy, Nix command copy, development build no-op, unknown source fallback?
- Should operators be able to skip/snooze a specific stable patch version, and where should that skip state be stored?
