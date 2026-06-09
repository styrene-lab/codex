# Agent Guidance

This repository is Flynt. Keep release-visible surfaces in sync when preparing a version bump.

## Release surface checklist

For every Flynt release or patch train update:

1. Update the workspace version in `Cargo.toml`.
2. Update the Flynt ACP extension version in `crates/flynt-agent/manifest.toml`.
3. Update the static site package version in `site/package.json` and refresh `site/package-lock.json`.
4. Add or update the matching top-level section in `CHANGELOG.md`.
5. Update public site copy under `site/src/pages/**` when the release changes operator-facing behavior, installation posture, sync posture, project-state layout, or agent/runtime behavior.
6. Run `python3 scripts/check-release-surfaces.py` before committing.
7. Run `python3 scripts/check-site-screenshots.py` when screenshot references, screenshot manifests, docs pages, or generated screenshot assets change.
8. Run `npm --prefix site run build` after site edits.

Do not let the site drift behind release notes. If a change is important enough for `CHANGELOG.md`, decide explicitly whether it needs a public-site/docs update. If not, leave the site unchanged for a reason, not by omission.

## Screenshot documentation

The public site has a screenshot contract for demo-vault-backed visuals:

- docs/site data: `site/src/data/screenshots.json`
- capture scenarios: `site/screenshots/demo-vault-scenarios.json`
- generated or placeholder assets: `site/public/screenshots/`
- validator: `scripts/check-site-screenshots.py`

When adding a screenshot, add it to both JSON files, provide alt text and a caption, add a placeholder asset or generated capture under `site/public/screenshots/`, then run the screenshot validator. Prefer generated screenshots from the demo vault when the app capture harness is available; placeholders are acceptable only to keep the site layout stable while automation is being built.

## Site deployment

The production site source lives in `site/` and deploys through `.github/workflows/deploy-site.yml`. The site is curated public documentation, not a dump of internal design notes.

## release-plz

`release-plz` is already wired but intentionally dormant until Flynt crates are ready for crates.io publishing:

- workflow: `.github/workflows/release-plz.yml`
- config: `release-plz.toml`

Do not assume release-plz updates application release notes today. Until it is enabled, maintain `CHANGELOG.md` manually and use `scripts/check-release-surfaces.py` as the repository-level release-surface guard.

## Local launch discipline

On macOS, do not use `cargo run -p flynt-app` for operator-facing local validation. Raw binaries show the generic `exec` Dock icon and repeatedly regress release screenshots. For local GUI validation, launch through the app bundle wrapper:

```bash
scripts/launch-local-app.sh fixtures/demo-vault
```

That script runs `dx build --macos -p flynt-app --bin flynt`, uses the Dioxus-generated bundle at `target/dx/flynt/debug/macos/Flynt.app`, installs `icon.icns`/`AppIcon.icns`, and launches with `open ... --args --project ...`. Use raw `cargo run` only for log-only/debug runs where the Dock icon is irrelevant, and say so explicitly.

