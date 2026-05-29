+++
id = "6600e402-3c35-4ee0-b744-a211fb1d4f0d"
kind = "design_node"

[data]
title = "Flynt UI gaps and issues"
status = "exploring"
issue_type = "backlog"
priority = 2
parent = "flynt-root"
dependencies = []
open_questions = [
  "[assumption] The terminal paste fix covers ordinary Cmd+V/browser paste paths, but not full clipboard/selection behavior.",
  "[assumption] Settings-only diagnostics are sufficient for now, even though ACP startup failures may need a more prominent rail-level preflight.",
  "Should UI gaps be tracked as one backlog node or split by surface once implementation begins?",
]
tags = ["ui", "ux", "terminal", "design-board", "agent-rail", "settings", "navigation", "quality"]
+++

# Flynt UI gaps and issues

## Overview

This node collects known UI/UX gaps discovered while hardening Flynt's embedded Omegon/ACP workflows, design surfaces, and terminal panel. It exists as a single planning backlog so small paper cuts do not disappear into ad hoc chat history.

The goal is not to over-design every fix now. The goal is to keep an honest inventory, classify severity, and branch focused implementation nodes when a cluster is ready.

Related nodes: [[host-actions-permission-review]], [[terminal-validation-actions]], [[design-sidebar-organization]], [[design-board-visual-substrate]], [[flynt-acp-deployment-diagnostics]].

## Current inventory

### Terminal panel

- **Initial session selection** — fixed in the first pass by selecting and snapshotting the auto-created terminal immediately.
- **Paste into terminal** — fixed for ordinary paste by handling `onpaste` and reading `navigator.clipboard.readText()`.
- **Remaining gap: copy/selection model** — there is no real terminal text selection/copy interaction yet.
- **Remaining gap: mouse support** — mouse reporting, selection drag, and terminal-app mouse interactions are not implemented.
- **Remaining gap: IME/composition** — non-ASCII composition input has not been validated.
- **Remaining gap: xterm-class behavior** — current UI is a renderer over snapshots, not a full xterm-compatible frontend.
- **Remaining gap: terminal preflight/health** — no visible diagnosis for shell spawn failure beyond inline error text.

### Agent rail / ACP

- **Deployment metadata ingestion** — implemented: ACP metadata can be consumed and classified.
- **Remaining gap: preflight prominence** — deployment/CLI failures are visible in Settings, but agent rail startup should surface blocking diagnostics before the operator sends a prompt.
- **Remaining gap: metadata persistence semantics** — deployment metadata is runtime-only; after app restart, Settings is unknown until ACP initializes.
- **Remaining gap: profile mismatch remediation** — diagnostics identify mismatch but do not offer a one-click repair/apply profile action.
- **Remaining gap: command consolidation migration** — CLI probe is heuristic (`omegon acp --help`) until Omegon exposes a machine-readable compatibility endpoint.

### Settings / diagnostics

- **Runtime diagnostic card** — implemented for deployment and CLI contract status.
- **Remaining gap: provenance display** — settings should show where profile/skills/extensions were resolved from: project override, app bundle, user Armory, or remote catalog.
- **Remaining gap: reload/refresh control** — no explicit button to rerun CLI probe or reload deployment manifest.
- **Remaining gap: actionable repairs** — diagnostics should distinguish observe-only status from fixable status and provide safe actions.

### Design mode / design board UX

- **Design artifact listing** — implemented for boards, drawings, and flows.
- **Design artifact creation buttons** — implemented for board/drawing/flow.
- **Remaining gap: coordinated deletion** — deleting wrapper-backed artifacts still needs safe wrapper+backing-file deletion, tab closure, and event emission.
- **Remaining gap: artifact row styling** — artifact rows should be audited against the rest of sidebar/list styling.
- **Remaining gap: content-based wrapper detection** — current listing is path-based; robust detection should inspect wrapper content or stored kind metadata.
- **Remaining gap: design surface maturity messaging** — central Design view and panel copy should continue to avoid implying that experimental surfaces are polished.
- **Remaining gap: create flow details** — new flow creation uses a timestamped default graph; future UX should support naming and placement choices.

### Navigation / sidebar

- **Design sidebar organization** — active design node exists: [[design-sidebar-organization]].
- **Remaining gap: false affordances** — rows rendered as buttons without actions should be removed or wired. The design surface profile rows were identified as a specific risk.
- **Remaining gap: selected state consistency** — route/sidebar/tab selected states should be audited after adding Terminal and Design paths.
- **Remaining gap: empty-state honesty** — empty states should describe what is actually available now, not future intent.

### App identity / build visibility

- **Bundled icon/version** — implemented for macOS app bundle and toolbar version label.
- **Remaining gap: raw cargo-run identity** — running the raw binary may still show generic macOS process identity; the recommended dev path is the bundled app when testing icon/version behavior.
- **Remaining gap: build metadata source** — toolbar currently shows semver + build hash; release/channel metadata may need to join this once update channels mature.

## Severity map

### Blocker

- ACP/agent rail should not silently fail when CLI contract or deployment profile is incompatible.
- Wrapper-backed delete must not imply complete deletion while only deleting wrapper markdown.

### High

- Agent rail preflight should surface deployment/CLI diagnostics before prompt submission.
- Terminal input must reliably handle ordinary keyboard and paste paths.
- Design artifact discovery should avoid stale canvas terminology and false positives.

### Medium

- Settings diagnostics need provenance and repair actions.
- Terminal should gain copy/selection and better spawn failure display.
- Design rows should have consistent styling and selected states.

### Low

- Build/channel metadata polish.
- Empty-state copy cleanup.
- Fine-grained terminal mouse/IME behavior unless demanded by terminal-app workflows.

## Decisions so far

- Keep this as a single backlog node while gaps are still cross-cutting.
- Branch focused child nodes when a cluster is ready for implementation.
- Prefer small, directly validated fixes over a broad UI rewrite.
- Treat diagnostics as product UI, not developer-only logs.
- Treat wrapper-backed artifacts as compound resources; deletion/rename must eventually operate on both wrapper and backing file.

## Implementation candidates

1. **Agent rail preflight card**
   - Show deployment + CLI probe status in the agent rail before first prompt.
   - Disable or warn on prompt send if status is blocked.

2. **Terminal UX hardening**
   - Add copy/selection affordance.
   - Add explicit focus indicator.
   - Validate paste/keyboard behavior across macOS app bundle and raw dev run.

3. **Safe artifact delete**
   - Resolve wrapper-backed resources.
   - Delete wrapper + backing file together.
   - Close affected tabs.
   - Emit project events and reindex.

4. **Diagnostics repair actions**
   - Rerun CLI probe.
   - Regenerate deployment manifest.
   - Apply/select `flynt-agent` profile.

5. **Design surface copy/a11y cleanup**
   - Remove false buttons.
   - Align central Design view copy with current maturity model.
   - Audit empty states.
