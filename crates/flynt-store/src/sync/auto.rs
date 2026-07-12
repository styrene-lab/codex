//! Background auto-sync loop for git-backed projects.
//!
//! Periodically commits local changes and syncs with the remote.
//! Designed to keep phone and desktop projects in sync via a shared git repo.

use super::git::GitSync;
use super::planner::SyncRequest;
use super::runner::{BackgroundSyncRunner, SyncEvent, SyncPhase};
use super::save_quiescence::SaveQuiescenceTracker;
use super::vcs::{GitVcsAdapter, SyncOutcome};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::watch;
use tracing::{info, warn};

/// Status reported by the sync loop.
#[derive(Debug, Clone, PartialEq)]
pub enum AutoSyncStatus {
    Idle,
    Committing,
    Pulling,
    Pushing,
    WaitingForSaves,
    Conflict(Vec<String>),
    Error(String),
}

/// Handle to a running background sync loop. Drop to stop.
pub struct AutoSyncHandle {
    _cancel: watch::Sender<bool>,
}

/// Start a background auto-sync loop for a project.
///
/// - Commits any dirty files every `interval`
/// - Pulls from remote (fast-forward or merge)
/// - Pushes local commits to remote
/// - Reports status via the returned watch receiver
/// - Exponential backoff on repeated failures (capped at 10 minutes)
///
/// The loop runs until the handle is dropped.
pub fn start_auto_sync(
    project_root: PathBuf,
    remote: String,
    branch: String,
    interval: Duration,
    reindex: Option<Arc<dyn Fn() + Send + Sync>>,
    save_tracker: Option<SaveQuiescenceTracker>,
) -> (AutoSyncHandle, watch::Receiver<AutoSyncStatus>) {
    let (cancel_tx, cancel_rx) = watch::channel(false);
    let (status_tx, status_rx) = watch::channel(AutoSyncStatus::Idle);

    tokio::spawn(async move {
        let git = GitSync::new(project_root, &remote, &branch);
        let runner = BackgroundSyncRunner::new(GitVcsAdapter::new(git), branch.clone());
        let mut cancel = cancel_rx;
        let mut consecutive_failures: u32 = 0;
        let max_backoff = Duration::from_secs(600); // 10 minute cap

        loop {
            // Backoff on repeated failures: interval * 2^failures, capped
            let wait = if consecutive_failures > 0 {
                let backoff = interval.mul_f64(2.0_f64.powi(consecutive_failures.min(6) as i32));
                backoff.min(max_backoff)
            } else {
                interval
            };

            tokio::select! {
                _ = tokio::time::sleep(wait) => {},
                _ = cancel.changed() => {
                    if *cancel.borrow() { break; }
                }
            }

            let _ = status_tx.send(AutoSyncStatus::Committing);
            let request = SyncRequest {
                save_quiescence: save_tracker.as_ref().map(|tracker| tracker.current()),
                ..SyncRequest::auto_sync_tick()
            };
            match runner.run_once(request) {
                Ok(result) => {
                    for event in &result.events {
                        match event {
                            SyncEvent::PhaseChanged(SyncPhase::RefreshingUpstream)
                            | SyncEvent::PhaseChanged(SyncPhase::Observing)
                            | SyncEvent::PhaseChanged(SyncPhase::Preflight)
                            | SyncEvent::PhaseChanged(SyncPhase::ReconcilingLocalVcs) => {
                                let _ = status_tx.send(AutoSyncStatus::Pulling);
                            }
                            _ => {}
                        }
                    }

                    match result.outcome {
                        SyncOutcome::Blocked { blockers } => {
                            let conflicts = blockers
                                .into_iter()
                                .map(|blocker| format!("{blocker:?}"))
                                .collect::<Vec<_>>();
                            warn!(
                                "sync blocked: {:?}; stopping auto-sync until operator resolves it",
                                conflicts
                            );
                            let _ = status_tx.send(AutoSyncStatus::Conflict(conflicts));
                            break;
                        }
                        SyncOutcome::Failed { message, retryable } => {
                            consecutive_failures += 1;
                            warn!(
                                "sync failed (attempt {consecutive_failures}, retryable={retryable}): {message}"
                            );
                            let _ = status_tx.send(AutoSyncStatus::Error(message));
                            if !retryable {
                                break;
                            }
                            continue;
                        }
                        SyncOutcome::Synced { pulled, pushed, .. } => {
                            if pulled && let Some(ref cb) = reindex {
                                cb();
                            }
                            if pushed {
                                let _ = status_tx.send(AutoSyncStatus::Pushing);
                            }
                            if consecutive_failures > 0 {
                                info!("sync recovered after {consecutive_failures} failures");
                            }
                            consecutive_failures = 0;
                            let _ = status_tx.send(AutoSyncStatus::Idle);
                        }
                        SyncOutcome::Noop | SyncOutcome::UpstreamChecked { .. } => {
                            consecutive_failures = 0;
                            let _ = status_tx.send(AutoSyncStatus::Idle);
                        }
                        SyncOutcome::Deferred { .. } => {
                            consecutive_failures = 0;
                            let _ = status_tx.send(AutoSyncStatus::WaitingForSaves);
                        }
                        SyncOutcome::Committed { .. }
                        | SyncOutcome::FastForwarded { .. }
                        | SyncOutcome::Pushed { .. } => {
                            consecutive_failures = 0;
                            let _ = status_tx.send(AutoSyncStatus::Idle);
                        }
                    }
                }
                Err(e) => {
                    consecutive_failures += 1;
                    warn!("sync runner failed (attempt {consecutive_failures}): {e}");
                    let _ = status_tx.send(AutoSyncStatus::Error(format!("sync: {e}")));
                }
            }
        }

        info!("auto-sync loop stopped");
    });

    (AutoSyncHandle { _cancel: cancel_tx }, status_rx)
}
