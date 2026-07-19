# Daily-driver isolation

Flynt has three explicit runtime identities. They are separate applications, not modes inside one mutable installation.

| Identity | Bundle ID | Default data | Purpose |
|---|---|---|---|
| **Flynt** | `io.styrene.flynt` | Canonical operator vault | Stable daily driver; installed only from signed releases |
| **Flynt Candidate** | `io.styrene.flynt.candidate` | Timestamped writable snapshot | Release-like validation and promotion gate |
| **Flynt Dev** | `io.styrene.flynt.dev` | `fixtures/demo-vault` | Fast source iteration |

The toolbar and native window title expose the active identity. Each non-Stable launcher also passes a dedicated `FLYNT_LAUNCHER_PROFILE`, preventing recent-project, update-channel, and skipped-update state from leaking between identities.

## Non-negotiable boundary

Development scripts do not install over `/Applications/Flynt.app`, do not terminate Flynt by process name, and do not default to the canonical vault. Stable remains the operator's dependable application while Candidate and Dev run concurrently.

## Development loop

```sh
scripts/launch-local-app.sh
```

This builds and launches **Flynt Dev** against `fixtures/demo-vault`. To use another disposable fixture, pass it explicitly:

```sh
scripts/launch-local-app.sh /path/to/disposable-vault
```

Do not point Dev at a canonical daily-driver vault. The script permits an explicit path because compatibility testing sometimes requires a prepared fixture; that choice is visible at invocation rather than inherited from Stable state.

## Candidate loop

```sh
scripts/launch-candidate.sh /path/to/canonical-vault
```

The launcher:

1. Copies the source vault to `~/.local/share/flynt/candidates/<name>-candidate-<UTC timestamp>`.
2. Excludes `.git`, preventing Candidate from pushing from the canonical vault's worktree.
3. Records snapshot provenance in `.flynt-candidate-source.json`.
4. Builds a release-profile **Flynt Candidate** bundle.
5. Launches it with an isolated launcher profile against the snapshot only.

Create a snapshot without launching:

```sh
scripts/prepare-candidate.sh /path/to/canonical-vault [snapshot-parent]
```

Candidate snapshots are writable test artifacts, not backups. Promotion means publishing and installing the tested commit through the signed Stable release path; it never means copying Candidate state back over the canonical vault automatically.

## Stable promotion gate

Before replacing the daily driver:

1. Run targeted Rust tests and `cargo check -p flynt-app`.
2. Run `python3 scripts/test-daily-driver-isolation.py`.
3. Validate the Candidate bundle against a fresh snapshot.
4. Verify startup, note editing, task mutation, search, agent startup, sync status, and restart.
5. Build/sign/notarize through the normal release process.
6. Install the signed Stable artifact deliberately.
7. Retain the previous Stable installer until the new build has survived normal daily use.

## Recovery posture

The canonical vault remains ordinary project files. Candidate preparation is copy-before-test. Stable upgrades remain user-confirmed and signed. If a release regresses, quit it and reinstall the retained prior Stable artifact; do not attempt repair using a development bundle.
