//! VCS adapter boundary for guarded sync operations.
//!
//! The first implementation wraps the existing `GitSync` diagnostics and the
//! pure planner model. Future runner code should depend on `SyncVcs` instead of
//! calling `GitSync` mutation methods directly.

use anyhow::Result;
use flynt_core::sync::SyncBackend;

use super::git::{AutoCommitResult, GitSync, SyncDiagnostic};
use super::planner::{
    FreshnessStatus, GitRepositoryState, SyncBlocker, TagFetchPolicy, UpstreamFreshness,
    UpstreamRelation,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VcsFlavor {
    GitOnly,
    JjColocated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum VcsReconcileOutcome {
    NotNeeded,
    AlreadyAligned,
    FastForwardedBranch {
        branch: String,
        from: String,
        to: String,
    },
    ExportedJjState,
    Blocked(Vec<SyncBlocker>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncOutcome {
    Noop,
    UpstreamChecked {
        freshness: FreshnessStatus,
        relation: UpstreamRelation,
    },
    Committed {
        commit: String,
        files: usize,
    },
    FastForwarded {
        from: String,
        to: String,
    },
    Pushed {
        commit: String,
    },
    Synced {
        committed: bool,
        pulled: bool,
        pushed: bool,
    },
    Blocked {
        blockers: Vec<SyncBlocker>,
    },
    Failed {
        message: String,
        retryable: bool,
    },
    Deferred {
        reason: super::planner::SyncDeferReason,
    },
}

pub trait SyncVcs {
    fn flavor(&self) -> VcsFlavor;
    fn reconcile_local_vcs(&self, branch: &str) -> Result<VcsReconcileOutcome>;
    fn diagnostic(&self) -> Result<GitRepositoryState>;
    fn check_upstream(&self, tag_policy: TagFetchPolicy) -> Result<UpstreamFreshness>;
    fn list_refs(&self, tag_policy: TagFetchPolicy) -> Result<super::planner::GitRefInventory>;
    fn auto_commit_filtered(&self, message: &str) -> Result<AutoCommitResult>;
    fn pull_fast_forward(&self) -> Result<SyncOutcome>;
    fn push(&self) -> Result<SyncOutcome>;
    fn supports_auto_commit(&self) -> bool;
}

#[derive(Debug, Clone)]
pub struct GitVcsAdapter {
    git: GitSync,
}

impl GitVcsAdapter {
    pub fn new(git: GitSync) -> Self {
        Self { git }
    }

    pub fn git(&self) -> &GitSync {
        &self.git
    }
}

impl SyncVcs for GitVcsAdapter {
    fn flavor(&self) -> VcsFlavor {
        VcsFlavor::GitOnly
    }

    fn reconcile_local_vcs(&self, _branch: &str) -> Result<VcsReconcileOutcome> {
        Ok(VcsReconcileOutcome::NotNeeded)
    }

    fn diagnostic(&self) -> Result<GitRepositoryState> {
        diagnostic_to_repository_state(self.git.diagnostic()?)
    }

    fn check_upstream(&self, tag_policy: TagFetchPolicy) -> Result<UpstreamFreshness> {
        self.git.check_upstream(tag_policy)
    }

    fn list_refs(&self, tag_policy: TagFetchPolicy) -> Result<super::planner::GitRefInventory> {
        self.git.list_refs(tag_policy)
    }

    fn auto_commit_filtered(&self, message: &str) -> Result<AutoCommitResult> {
        self.git.auto_commit(message)
    }

    fn pull_fast_forward(&self) -> Result<SyncOutcome> {
        let result = self.git.pull()?;
        if result.conflicts.is_empty() {
            Ok(SyncOutcome::Synced {
                committed: false,
                pulled: result.files_pulled > 0,
                pushed: false,
            })
        } else {
            Ok(SyncOutcome::Blocked {
                blockers: vec![SyncBlocker::Diverged {
                    ahead: 0,
                    behind: 0,
                }],
            })
        }
    }

    fn push(&self) -> Result<SyncOutcome> {
        let result = self.git.push()?;
        Ok(SyncOutcome::Synced {
            committed: false,
            pulled: false,
            pushed: result.files_pushed > 0,
        })
    }

    fn supports_auto_commit(&self) -> bool {
        true
    }
}

pub fn diagnostic_to_repository_state(diagnostic: SyncDiagnostic) -> Result<GitRepositoryState> {
    let blockers = diagnostic
        .blockers
        .into_iter()
        .map(|message| SyncBlocker::OperationInProgress { state: message })
        .collect();

    Ok(GitRepositoryState {
        dirty_eligible_files: diagnostic.dirty_files.len(),
        auto_commit_supported: true,
        save_quiescence: super::planner::SaveQuiescence::Idle,
        upstream: {
            let mut upstream = upstream_from_counts(
                diagnostic.ahead,
                diagnostic.behind,
                diagnostic.remote_ref_available,
            );
            upstream.freshness = FreshnessStatus::Unknown;
            upstream
        },
        refs: super::planner::GitRefInventory::default(),
        blockers,
    })
}

fn upstream_from_counts(
    ahead: Option<usize>,
    behind: Option<usize>,
    remote_ref_available: bool,
) -> UpstreamFreshness {
    let relation = if !remote_ref_available {
        UpstreamRelation::MissingRemoteRef
    } else {
        match (ahead.unwrap_or(0), behind.unwrap_or(0)) {
            (0, 0) => UpstreamRelation::InSync,
            (ahead, 0) => UpstreamRelation::Ahead { ahead },
            (0, behind) => UpstreamRelation::OutOfDate { behind },
            (ahead, behind) => UpstreamRelation::Diverged { ahead, behind },
        }
    };

    let mut freshness = UpstreamFreshness::new(FreshnessStatus::Fresh, relation);
    freshness.ahead = ahead;
    freshness.behind = behind;
    freshness
}

#[cfg(test)]
mod tests {
    use super::*;

    fn diagnostic(ahead: Option<usize>, behind: Option<usize>) -> SyncDiagnostic {
        SyncDiagnostic {
            backend: "git".into(),
            remote: "origin".into(),
            branch: "main".into(),
            head: Some("abc".into()),
            dirty_files: Vec::new(),
            ahead,
            behind,
            remote_ref_available: true,
            detached_head: false,
            repository_state: "clean".into(),
            index_conflicts: Vec::new(),
            blockers: Vec::new(),
        }
    }

    #[test]
    fn diagnostic_maps_in_sync_relation() {
        let state = diagnostic_to_repository_state(diagnostic(Some(0), Some(0))).unwrap();
        assert_eq!(state.upstream.relation, UpstreamRelation::InSync);
    }

    #[test]
    fn diagnostic_maps_ahead_relation() {
        let state = diagnostic_to_repository_state(diagnostic(Some(2), Some(0))).unwrap();
        assert_eq!(
            state.upstream.relation,
            UpstreamRelation::Ahead { ahead: 2 }
        );
    }

    #[test]
    fn diagnostic_maps_out_of_date_relation() {
        let state = diagnostic_to_repository_state(diagnostic(Some(0), Some(3))).unwrap();
        assert_eq!(
            state.upstream.relation,
            UpstreamRelation::OutOfDate { behind: 3 }
        );
    }

    #[test]
    fn diagnostic_maps_diverged_relation() {
        let state = diagnostic_to_repository_state(diagnostic(Some(1), Some(2))).unwrap();
        assert_eq!(
            state.upstream.relation,
            UpstreamRelation::Diverged {
                ahead: 1,
                behind: 2,
            }
        );
    }

    #[test]
    fn diagnostic_maps_missing_remote_ref() {
        let mut d = diagnostic(None, None);
        d.remote_ref_available = false;
        let state = diagnostic_to_repository_state(d).unwrap();
        assert_eq!(state.upstream.relation, UpstreamRelation::MissingRemoteRef);
    }
}
