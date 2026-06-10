//! Background sync runner skeleton.
//!
//! This runner centralizes sync operation ordering around the pure planner and
//! VCS adapter boundary. The first slice plans safely and reports the selected
//! action; later slices will execute mutating plans through `SyncVcs`.

use anyhow::Result;

use super::planner::{SyncPlan, SyncPreflight, SyncRequest, SyncTrigger, plan_sync};
use super::vcs::{SyncOutcome, SyncVcs, VcsReconcileOutcome};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncPhase {
    ReconcilingLocalVcs,
    Observing,
    RefreshingUpstream,
    Preflight,
    WaitingForSaves,
    Complete,
    Blocked,
    Failed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncEvent {
    PhaseChanged(SyncPhase),
    VcsReconciled(VcsReconcileOutcome),
    Diagnostic(SyncPreflight),
    Completed(SyncOutcome),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncRunResult {
    pub preflight: SyncPreflight,
    pub outcome: SyncOutcome,
    pub events: Vec<SyncEvent>,
}

pub struct BackgroundSyncRunner<V: SyncVcs> {
    vcs: V,
    branch: String,
}

impl<V: SyncVcs> BackgroundSyncRunner<V> {
    pub fn new(vcs: V, branch: impl Into<String>) -> Self {
        Self {
            vcs,
            branch: branch.into(),
        }
    }

    pub fn run_once(&self, request: SyncRequest) -> Result<SyncRunResult> {
        let mut events = Vec::new();
        events.push(SyncEvent::PhaseChanged(SyncPhase::ReconcilingLocalVcs));
        let reconciliation = self.vcs.reconcile_local_vcs(&self.branch)?;
        let blocked_reconciliation = match &reconciliation {
            VcsReconcileOutcome::Blocked(blockers) => Some(blockers.clone()),
            _ => None,
        };
        events.push(SyncEvent::VcsReconciled(reconciliation));
        if let Some(blockers) = blocked_reconciliation {
            let preflight = SyncPreflight {
                safe_to_sync: false,
                blockers: blockers.clone(),
                warnings: Vec::new(),
                next_action: SyncPlan::Blocked,
            };
            let outcome = SyncOutcome::Blocked { blockers };
            events.push(SyncEvent::PhaseChanged(SyncPhase::Blocked));
            events.push(SyncEvent::Completed(outcome.clone()));
            return Ok(SyncRunResult {
                preflight,
                outcome,
                events,
            });
        }

        events.push(SyncEvent::PhaseChanged(SyncPhase::Observing));
        let mut state = self.vcs.diagnostic()?;
        state.auto_commit_supported = self.vcs.supports_auto_commit();
        if let Some(save_quiescence) = request.save_quiescence.clone() {
            state.save_quiescence = save_quiescence;
        }
        let mut preflight = plan_sync(&state, &request);

        if preflight.next_action == SyncPlan::RefreshUpstream {
            events.push(SyncEvent::PhaseChanged(SyncPhase::RefreshingUpstream));
            state.upstream = self
                .vcs
                .check_upstream(super::planner::TagFetchPolicy::AutoFollow)?;
            preflight = plan_sync(&state, &request);
        }

        events.push(SyncEvent::PhaseChanged(SyncPhase::Preflight));
        events.push(SyncEvent::Diagnostic(preflight.clone()));

        let outcome = match preflight.next_action {
            SyncPlan::WaitForSaves => SyncOutcome::Deferred {
                reason: match state.save_quiescence.clone() {
                    super::planner::SaveQuiescence::Active { paths } => {
                        super::planner::SyncDeferReason::AutosaveActive { paths }
                    }
                    super::planner::SaveQuiescence::RecentlySaved => {
                        super::planner::SyncDeferReason::AutosaveRecentlyCompleted
                    }
                    super::planner::SaveQuiescence::Idle => {
                        super::planner::SyncDeferReason::AutosaveRecentlyCompleted
                    }
                },
            },
            SyncPlan::Blocked => SyncOutcome::Blocked {
                blockers: preflight.blockers.clone(),
            },
            SyncPlan::Noop => SyncOutcome::Noop,
            SyncPlan::RefreshUpstream => SyncOutcome::UpstreamChecked {
                freshness: state.upstream.freshness,
                relation: state.upstream.relation,
            },
            SyncPlan::CommitOnly => {
                let commit = self.vcs.auto_commit_filtered("[flynt] auto-sync")?;
                match commit {
                    Some(commit) => SyncOutcome::Committed { commit, files: 0 },
                    None => SyncOutcome::Synced {
                        committed: true,
                        pulled: false,
                        pushed: false,
                    },
                }
            }
            SyncPlan::CommitThenPush => {
                let _commit = self.vcs.auto_commit_filtered("[flynt] auto-sync")?;
                events.push(SyncEvent::PhaseChanged(SyncPhase::RefreshingUpstream));
                state.upstream = self
                    .vcs
                    .check_upstream(super::planner::TagFetchPolicy::AutoFollow)?;
                preflight = plan_sync(&state, &request);
                events.push(SyncEvent::Diagnostic(preflight.clone()));
                if !preflight.safe_to_sync {
                    SyncOutcome::Blocked {
                        blockers: preflight.blockers.clone(),
                    }
                } else {
                    self.vcs.push()?
                }
            }
            SyncPlan::PullFastForward => self.vcs.pull_fast_forward()?,
            SyncPlan::PushOnly => self.vcs.push()?,
            SyncPlan::PullThenPush => {
                let pull = self.vcs.pull_fast_forward()?;
                if matches!(
                    pull,
                    SyncOutcome::Blocked { .. } | SyncOutcome::Failed { .. }
                ) {
                    pull
                } else {
                    self.vcs.push()?
                }
            }
        };

        events.push(SyncEvent::PhaseChanged(
            if matches!(outcome, SyncOutcome::Blocked { .. }) {
                SyncPhase::Blocked
            } else if matches!(outcome, SyncOutcome::Deferred { .. }) {
                SyncPhase::WaitingForSaves
            } else {
                SyncPhase::Complete
            },
        ));
        events.push(SyncEvent::Completed(outcome.clone()));

        Ok(SyncRunResult {
            preflight,
            outcome,
            events,
        })
    }

    pub fn refresh_only(&self) -> Result<SyncRunResult> {
        self.run_once(SyncRequest::refresh_only())
    }

    pub fn manual_sync_plan(&self) -> Result<SyncRunResult> {
        self.run_once(SyncRequest::manual_sync())
    }

    pub fn auto_sync_plan(&self) -> Result<SyncRunResult> {
        self.run_once(SyncRequest::auto_sync_tick())
    }
}

impl SyncRequest {
    pub fn is_refresh_only(&self) -> bool {
        self.trigger == SyncTrigger::RefreshOnly
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sync::planner::{
        FreshnessStatus, GitRepositoryState, SyncBlocker, TagFetchPolicy, UpstreamFreshness,
        UpstreamRelation,
    };
    use crate::sync::vcs::{SyncVcs, VcsFlavor};
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    #[derive(Clone)]
    struct FakeVcs {
        diagnostic: GitRepositoryState,
        upstream: UpstreamFreshness,
        auto_commit_supported: bool,
        reconcile: VcsReconcileOutcome,
        commit_count: Arc<AtomicUsize>,
        pull_count: Arc<AtomicUsize>,
        push_count: Arc<AtomicUsize>,
    }

    impl SyncVcs for FakeVcs {
        fn flavor(&self) -> VcsFlavor {
            VcsFlavor::GitOnly
        }

        fn reconcile_local_vcs(&self, _branch: &str) -> Result<VcsReconcileOutcome> {
            Ok(self.reconcile.clone())
        }

        fn diagnostic(&self) -> Result<GitRepositoryState> {
            Ok(self.diagnostic.clone())
        }

        fn check_upstream(&self, _tag_policy: TagFetchPolicy) -> Result<UpstreamFreshness> {
            Ok(self.upstream.clone())
        }

        fn auto_commit_filtered(&self, _message: &str) -> Result<Option<String>> {
            self.commit_count.fetch_add(1, Ordering::SeqCst);
            Ok(Some("commit".into()))
        }

        fn pull_fast_forward(&self) -> Result<SyncOutcome> {
            self.pull_count.fetch_add(1, Ordering::SeqCst);
            Ok(SyncOutcome::Synced {
                committed: false,
                pulled: true,
                pushed: false,
            })
        }

        fn push(&self) -> Result<SyncOutcome> {
            self.push_count.fetch_add(1, Ordering::SeqCst);
            Ok(SyncOutcome::Synced {
                committed: false,
                pulled: false,
                pushed: true,
            })
        }

        fn supports_auto_commit(&self) -> bool {
            self.auto_commit_supported
        }
    }

    fn fake(diagnostic: GitRepositoryState, upstream: UpstreamFreshness) -> FakeVcs {
        FakeVcs {
            diagnostic,
            upstream,
            auto_commit_supported: true,
            reconcile: VcsReconcileOutcome::NotNeeded,
            commit_count: Arc::new(AtomicUsize::new(0)),
            pull_count: Arc::new(AtomicUsize::new(0)),
            push_count: Arc::new(AtomicUsize::new(0)),
        }
    }

    #[test]
    fn runner_defers_when_autosave_is_active() {
        let mut diagnostic =
            GitRepositoryState::dirty(FreshnessStatus::Fresh, UpstreamRelation::InSync);
        diagnostic.save_quiescence = crate::sync::planner::SaveQuiescence::Active {
            paths: vec!["notes/today.md".into()],
        };
        let runner = BackgroundSyncRunner::new(
            fake(
                diagnostic,
                UpstreamFreshness::new(FreshnessStatus::Fresh, UpstreamRelation::InSync),
            ),
            "main",
        );

        let result = runner.manual_sync_plan().unwrap();

        assert_eq!(result.preflight.next_action, SyncPlan::WaitForSaves);
        assert!(matches!(result.outcome, SyncOutcome::Deferred { .. }));
        assert!(
            result
                .events
                .contains(&SyncEvent::PhaseChanged(SyncPhase::WaitingForSaves))
        );
    }

    #[test]
    fn runner_refreshes_stale_upstream_before_planning() {
        let diagnostic =
            GitRepositoryState::dirty(FreshnessStatus::Stale, UpstreamRelation::InSync);
        let upstream = UpstreamFreshness::new(FreshnessStatus::Fresh, UpstreamRelation::InSync);
        let runner = BackgroundSyncRunner::new(fake(diagnostic, upstream), "main");

        let result = runner.manual_sync_plan().unwrap();

        assert_eq!(result.preflight.next_action, SyncPlan::CommitThenPush);
        assert!(
            result
                .events
                .contains(&SyncEvent::PhaseChanged(SyncPhase::RefreshingUpstream))
        );
    }

    #[test]
    fn runner_blocks_reconciliation_blockers_before_diagnostic() {
        let mut vcs = fake(
            GitRepositoryState::clean(FreshnessStatus::Fresh, UpstreamRelation::InSync),
            UpstreamFreshness::new(FreshnessStatus::Fresh, UpstreamRelation::InSync),
        );
        vcs.reconcile = VcsReconcileOutcome::Blocked(vec![SyncBlocker::JjUnavailable]);
        let runner = BackgroundSyncRunner::new(vcs, "main");

        let result = runner.manual_sync_plan().unwrap();

        assert_eq!(result.preflight.next_action, SyncPlan::Blocked);
        assert!(matches!(result.outcome, SyncOutcome::Blocked { .. }));
    }

    #[test]
    fn runner_blocks_when_adapter_auto_commit_unsupported() {
        let mut vcs = fake(
            GitRepositoryState::dirty(FreshnessStatus::Fresh, UpstreamRelation::InSync),
            UpstreamFreshness::new(FreshnessStatus::Fresh, UpstreamRelation::InSync),
        );
        vcs.auto_commit_supported = false;
        let runner = BackgroundSyncRunner::new(vcs, "main");

        let result = runner.manual_sync_plan().unwrap();

        assert_eq!(result.preflight.next_action, SyncPlan::Blocked);
        assert!(
            result
                .preflight
                .blockers
                .contains(&SyncBlocker::JjAutoCommitUnsupported)
        );
    }

    #[test]
    fn runner_executes_pull_fast_forward_plan() {
        let vcs = fake(
            GitRepositoryState::clean(
                FreshnessStatus::Fresh,
                UpstreamRelation::OutOfDate { behind: 1 },
            ),
            UpstreamFreshness::new(FreshnessStatus::Fresh, UpstreamRelation::InSync),
        );
        let pull_count = vcs.pull_count.clone();
        let runner = BackgroundSyncRunner::new(vcs, "main");

        let result = runner.manual_sync_plan().unwrap();

        assert_eq!(pull_count.load(Ordering::SeqCst), 1);
        assert!(matches!(
            result.outcome,
            SyncOutcome::Synced { pulled: true, .. }
        ));
    }

    #[test]
    fn runner_executes_push_only_plan() {
        let vcs = fake(
            GitRepositoryState::clean(FreshnessStatus::Fresh, UpstreamRelation::Ahead { ahead: 1 }),
            UpstreamFreshness::new(FreshnessStatus::Fresh, UpstreamRelation::InSync),
        );
        let push_count = vcs.push_count.clone();
        let runner = BackgroundSyncRunner::new(vcs, "main");

        let result = runner.manual_sync_plan().unwrap();

        assert_eq!(push_count.load(Ordering::SeqCst), 1);
        assert!(matches!(
            result.outcome,
            SyncOutcome::Synced { pushed: true, .. }
        ));
    }

    #[test]
    fn runner_commit_then_push_rechecks_upstream_before_push() {
        let vcs = fake(
            GitRepositoryState::dirty(FreshnessStatus::Fresh, UpstreamRelation::InSync),
            UpstreamFreshness::new(FreshnessStatus::Fresh, UpstreamRelation::InSync),
        );
        let commit_count = vcs.commit_count.clone();
        let push_count = vcs.push_count.clone();
        let runner = BackgroundSyncRunner::new(vcs, "main");

        let result = runner.manual_sync_plan().unwrap();

        assert_eq!(commit_count.load(Ordering::SeqCst), 1);
        assert_eq!(push_count.load(Ordering::SeqCst), 1);
        assert!(matches!(
            result.outcome,
            SyncOutcome::Synced { pushed: true, .. }
        ));
        assert!(
            result
                .events
                .contains(&SyncEvent::PhaseChanged(SyncPhase::RefreshingUpstream))
        );
    }
}
