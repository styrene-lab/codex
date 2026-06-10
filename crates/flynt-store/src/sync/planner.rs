//! Pure planning model for guarded Git sync.
//!
//! This module intentionally avoids touching Git. It turns an observed sync
//! state plus request permissions into a safe next plan. Git mutation code must
//! treat `Blocked` as terminal until operator action.

use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncMode {
    Disabled,
    Manual,
    Auto { interval_secs: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DivergencePolicy {
    FastForwardOnly,
    MergeCommit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpstreamCheckPolicy {
    ManualOnly,
    OnSync,
    Periodic { interval_secs: u64 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagFetchPolicy {
    DoNotFetchTags,
    AutoFollow,
    AllTags,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitSyncConfig {
    pub remote: String,
    pub branch: String,
    pub mode: SyncMode,
    pub divergence_policy: DivergencePolicy,
    pub upstream_check: UpstreamCheckPolicy,
    pub tag_fetch_policy: TagFetchPolicy,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FreshnessStatus {
    Unknown,
    Fresh,
    Stale,
    Unreachable,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpstreamRelation {
    Unknown,
    InSync,
    OutOfDate { behind: usize },
    Ahead { ahead: usize },
    Diverged { ahead: usize, behind: usize },
    MissingRemoteRef,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpstreamFreshness {
    pub freshness: FreshnessStatus,
    pub relation: UpstreamRelation,
    pub last_checked_at: Option<DateTime<Utc>>,
    pub local_head: Option<String>,
    pub upstream_head: Option<String>,
    pub ahead: Option<usize>,
    pub behind: Option<usize>,
    pub stale_after_secs: u64,
}

impl UpstreamFreshness {
    pub fn new(freshness: FreshnessStatus, relation: UpstreamRelation) -> Self {
        Self {
            freshness,
            relation,
            last_checked_at: None,
            local_head: None,
            upstream_head: None,
            ahead: None,
            behind: None,
            stale_after_secs: 300,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitRepositoryState {
    pub dirty_eligible_files: usize,
    pub auto_commit_supported: bool,
    pub save_quiescence: SaveQuiescence,
    pub upstream: UpstreamFreshness,
    pub refs: GitRefInventory,
    pub blockers: Vec<SyncBlocker>,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GitRefInventory {
    pub local_branches: Vec<GitRefSummary>,
    pub remote_branches: Vec<GitRefSummary>,
    pub tags: Vec<GitRefSummary>,
    pub current: Option<GitRefSummary>,
    pub upstream: Option<GitRefSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GitRefSummary {
    pub name: String,
    pub full_ref: String,
    pub target: Option<String>,
    pub kind: GitRefKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GitRefKind {
    LocalBranch,
    RemoteBranch,
    Tag,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SaveQuiescence {
    Idle,
    Active { paths: Vec<String> },
    RecentlySaved,
}

impl GitRepositoryState {
    pub fn clean(freshness: FreshnessStatus, relation: UpstreamRelation) -> Self {
        Self {
            dirty_eligible_files: 0,
            auto_commit_supported: true,
            save_quiescence: SaveQuiescence::Idle,
            upstream: UpstreamFreshness::new(freshness, relation),
            refs: GitRefInventory::default(),
            blockers: Vec::new(),
        }
    }

    pub fn dirty(freshness: FreshnessStatus, relation: UpstreamRelation) -> Self {
        Self {
            dirty_eligible_files: 1,
            auto_commit_supported: true,
            save_quiescence: SaveQuiescence::Idle,
            upstream: UpstreamFreshness::new(freshness, relation),
            refs: GitRefInventory::default(),
            blockers: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncBlocker {
    NotGitRepository,
    MissingHead,
    DetachedHead,
    OperationInProgress {
        state: String,
    },
    IndexConflicts {
        paths: Vec<String>,
    },
    MissingRemote {
        remote: String,
    },
    MissingLocalBranch {
        branch: String,
    },
    MissingRemoteRef {
        remote: String,
        branch: String,
    },
    OutOfDate {
        behind: usize,
    },
    Diverged {
        ahead: usize,
        behind: usize,
    },
    DirtyOutOfDate {
        behind: usize,
    },
    DirtyDiverged {
        ahead: usize,
        behind: usize,
    },
    DirtyConflictMarkers {
        paths: Vec<String>,
    },
    UnsafePortableBoundary {
        paths: Vec<String>,
    },
    AuthenticationRequired,
    RemoteUnavailable {
        message: String,
    },
    JjUnavailable,
    JjGitDiverged {
        branch: String,
        git_head: String,
        jj_head: String,
    },
    JjGitExportFailed {
        message: String,
    },
    JjAutoCommitUnsupported,
    AutoCommitNotAllowed,
    PullNotAllowed,
    PushNotAllowed,
    AutosaveInProgress {
        paths: Vec<String>,
    },
    AutosaveRecentlyCompleted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncDeferReason {
    AutosaveActive { paths: Vec<String> },
    AutosaveRecentlyCompleted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SyncWarning {
    DirtyWorkingTree { count: usize },
    AutoSyncStopped,
    SingleWriterRecommended,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncPlan {
    WaitForSaves,
    RefreshUpstream,
    CommitOnly,
    CommitThenPush,
    PullFastForward,
    PushOnly,
    PullThenPush,
    Blocked,
    Noop,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncTrigger {
    Manual,
    Auto,
    RefreshOnly,
    Startup,
    BeforeClose,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncRequest {
    pub trigger: SyncTrigger,
    pub allow_commit: bool,
    pub allow_pull: bool,
    pub allow_push: bool,
    pub require_fresh_upstream: bool,
    pub save_quiescence: Option<SaveQuiescence>,
}

impl SyncRequest {
    pub fn manual_sync() -> Self {
        Self {
            trigger: SyncTrigger::Manual,
            allow_commit: true,
            allow_pull: true,
            allow_push: true,
            require_fresh_upstream: true,
            save_quiescence: None,
        }
    }

    pub fn auto_sync_tick() -> Self {
        Self {
            trigger: SyncTrigger::Auto,
            allow_commit: true,
            allow_pull: true,
            allow_push: true,
            require_fresh_upstream: true,
            save_quiescence: None,
        }
    }

    pub fn refresh_only() -> Self {
        Self {
            trigger: SyncTrigger::RefreshOnly,
            allow_commit: false,
            allow_pull: false,
            allow_push: false,
            require_fresh_upstream: true,
            save_quiescence: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncPreflight {
    pub safe_to_sync: bool,
    pub blockers: Vec<SyncBlocker>,
    pub warnings: Vec<SyncWarning>,
    pub next_action: SyncPlan,
}

pub fn plan_sync(state: &GitRepositoryState, request: &SyncRequest) -> SyncPreflight {
    let mut blockers = state.blockers.clone();
    let mut warnings = Vec::new();

    if state.dirty_eligible_files > 0 {
        warnings.push(SyncWarning::DirtyWorkingTree {
            count: state.dirty_eligible_files,
        });
    }

    if !blockers.is_empty() {
        return blocked(blockers, warnings);
    }

    match &state.save_quiescence {
        SaveQuiescence::Idle => {}
        SaveQuiescence::Active { paths: _ } => {
            return SyncPreflight {
                safe_to_sync: false,
                blockers,
                warnings,
                next_action: SyncPlan::WaitForSaves,
            };
        }
        SaveQuiescence::RecentlySaved => {
            return SyncPreflight {
                safe_to_sync: false,
                blockers,
                warnings,
                next_action: SyncPlan::WaitForSaves,
            };
        }
    }

    if request.require_fresh_upstream
        && matches!(
            state.upstream.freshness,
            FreshnessStatus::Unknown | FreshnessStatus::Stale
        )
    {
        return SyncPreflight {
            safe_to_sync: true,
            blockers,
            warnings,
            next_action: SyncPlan::RefreshUpstream,
        };
    }

    if matches!(state.upstream.freshness, FreshnessStatus::Unreachable) {
        blockers.push(SyncBlocker::RemoteUnavailable {
            message: "upstream is unreachable".into(),
        });
        return blocked(blockers, warnings);
    }

    let dirty = state.dirty_eligible_files > 0;
    match (dirty, state.upstream.relation) {
        (true, UpstreamRelation::OutOfDate { behind }) => {
            blockers.push(SyncBlocker::DirtyOutOfDate { behind });
            blocked(blockers, warnings)
        }
        (true, UpstreamRelation::Diverged { ahead, behind }) => {
            blockers.push(SyncBlocker::DirtyDiverged { ahead, behind });
            blocked(blockers, warnings)
        }
        (true, UpstreamRelation::InSync) => plan_dirty_in_sync(state, request, blockers, warnings),
        (true, UpstreamRelation::Ahead { .. }) => {
            plan_dirty_in_sync(state, request, blockers, warnings)
        }
        (true, UpstreamRelation::Unknown | UpstreamRelation::MissingRemoteRef) => {
            blockers.push(SyncBlocker::MissingRemoteRef {
                remote: "unknown".into(),
                branch: "unknown".into(),
            });
            blocked(blockers, warnings)
        }
        (false, UpstreamRelation::OutOfDate { behind: _ }) => {
            if request.allow_pull {
                ready(warnings, SyncPlan::PullFastForward)
            } else {
                blockers.push(SyncBlocker::PullNotAllowed);
                blocked(blockers, warnings)
            }
        }
        (false, UpstreamRelation::Ahead { ahead: _ }) => {
            if request.allow_push {
                ready(warnings, SyncPlan::PushOnly)
            } else {
                blockers.push(SyncBlocker::PushNotAllowed);
                blocked(blockers, warnings)
            }
        }
        (false, UpstreamRelation::Diverged { ahead, behind }) => {
            blockers.push(SyncBlocker::Diverged { ahead, behind });
            blocked(blockers, warnings)
        }
        (false, UpstreamRelation::InSync) => ready(warnings, SyncPlan::Noop),
        (false, UpstreamRelation::Unknown) => ready(warnings, SyncPlan::RefreshUpstream),
        (false, UpstreamRelation::MissingRemoteRef) => {
            blockers.push(SyncBlocker::MissingRemoteRef {
                remote: "unknown".into(),
                branch: "unknown".into(),
            });
            blocked(blockers, warnings)
        }
    }
}

fn plan_dirty_in_sync(
    state: &GitRepositoryState,
    request: &SyncRequest,
    mut blockers: Vec<SyncBlocker>,
    warnings: Vec<SyncWarning>,
) -> SyncPreflight {
    if !request.allow_commit {
        blockers.push(SyncBlocker::AutoCommitNotAllowed);
        return blocked(blockers, warnings);
    }
    if !state.auto_commit_supported {
        blockers.push(SyncBlocker::JjAutoCommitUnsupported);
        return blocked(blockers, warnings);
    }
    if request.allow_push {
        ready(warnings, SyncPlan::CommitThenPush)
    } else {
        ready(warnings, SyncPlan::CommitOnly)
    }
}

fn ready(warnings: Vec<SyncWarning>, next_action: SyncPlan) -> SyncPreflight {
    SyncPreflight {
        safe_to_sync: !matches!(next_action, SyncPlan::Blocked | SyncPlan::WaitForSaves),
        blockers: Vec::new(),
        warnings,
        next_action,
    }
}

fn blocked(blockers: Vec<SyncBlocker>, warnings: Vec<SyncWarning>) -> SyncPreflight {
    SyncPreflight {
        safe_to_sync: false,
        blockers,
        warnings,
        next_action: SyncPlan::Blocked,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(state: GitRepositoryState) -> SyncPreflight {
        plan_sync(&state, &SyncRequest::manual_sync())
    }

    #[test]
    fn autosave_active_defers_sync() {
        let mut state = GitRepositoryState::dirty(FreshnessStatus::Fresh, UpstreamRelation::InSync);
        state.save_quiescence = SaveQuiescence::Active {
            paths: vec!["notes/today.md".into()],
        };
        let preflight = plan(state);
        assert_eq!(preflight.next_action, SyncPlan::WaitForSaves);
        assert!(!preflight.safe_to_sync);
    }

    #[test]
    fn autosave_recently_completed_defers_sync() {
        let mut state = GitRepositoryState::dirty(FreshnessStatus::Fresh, UpstreamRelation::InSync);
        state.save_quiescence = SaveQuiescence::RecentlySaved;
        let preflight = plan(state);
        assert_eq!(preflight.next_action, SyncPlan::WaitForSaves);
        assert!(!preflight.safe_to_sync);
    }

    #[test]
    fn stale_upstream_refreshes_before_mutation() {
        let state = GitRepositoryState::dirty(FreshnessStatus::Stale, UpstreamRelation::InSync);
        assert_eq!(plan(state).next_action, SyncPlan::RefreshUpstream);
    }

    #[test]
    fn unknown_upstream_refreshes_before_mutation() {
        let state = GitRepositoryState::clean(FreshnessStatus::Unknown, UpstreamRelation::Unknown);
        assert_eq!(plan(state).next_action, SyncPlan::RefreshUpstream);
    }

    #[test]
    fn dirty_out_of_date_blocks() {
        let state = GitRepositoryState::dirty(
            FreshnessStatus::Fresh,
            UpstreamRelation::OutOfDate { behind: 1 },
        );
        let preflight = plan(state);
        assert_eq!(preflight.next_action, SyncPlan::Blocked);
        assert!(
            preflight
                .blockers
                .contains(&SyncBlocker::DirtyOutOfDate { behind: 1 })
        );
    }

    #[test]
    fn dirty_in_sync_commits_then_pushes_when_supported() {
        let state = GitRepositoryState::dirty(FreshnessStatus::Fresh, UpstreamRelation::InSync);
        assert_eq!(plan(state).next_action, SyncPlan::CommitThenPush);
    }

    #[test]
    fn dirty_in_sync_blocks_when_auto_commit_unsupported() {
        let mut state = GitRepositoryState::dirty(FreshnessStatus::Fresh, UpstreamRelation::InSync);
        state.auto_commit_supported = false;
        let preflight = plan(state);
        assert_eq!(preflight.next_action, SyncPlan::Blocked);
        assert!(
            preflight
                .blockers
                .contains(&SyncBlocker::JjAutoCommitUnsupported)
        );
    }

    #[test]
    fn clean_out_of_date_fast_forwards() {
        let state = GitRepositoryState::clean(
            FreshnessStatus::Fresh,
            UpstreamRelation::OutOfDate { behind: 2 },
        );
        assert_eq!(plan(state).next_action, SyncPlan::PullFastForward);
    }

    #[test]
    fn clean_ahead_pushes() {
        let state =
            GitRepositoryState::clean(FreshnessStatus::Fresh, UpstreamRelation::Ahead { ahead: 1 });
        assert_eq!(plan(state).next_action, SyncPlan::PushOnly);
    }

    #[test]
    fn diverged_blocks() {
        let state = GitRepositoryState::clean(
            FreshnessStatus::Fresh,
            UpstreamRelation::Diverged {
                ahead: 1,
                behind: 1,
            },
        );
        let preflight = plan(state);
        assert_eq!(preflight.next_action, SyncPlan::Blocked);
        assert!(preflight.blockers.contains(&SyncBlocker::Diverged {
            ahead: 1,
            behind: 1,
        }));
    }

    #[test]
    fn clean_in_sync_noops() {
        let state = GitRepositoryState::clean(FreshnessStatus::Fresh, UpstreamRelation::InSync);
        assert_eq!(plan(state).next_action, SyncPlan::Noop);
    }
}
