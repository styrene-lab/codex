//! Shared autosave quiescence tracker for sync coordination.
//!
//! Writers mark saves as active/recently-saved; the sync runner samples this
//! state and defers commit/pull/push until writes settle.

use std::collections::BTreeSet;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::planner::SaveQuiescence;

#[derive(Clone, Debug)]
pub struct SaveQuiescenceTracker {
    inner: Arc<Mutex<TrackerState>>,
    state: Arc<Mutex<SaveQuiescence>>,
    idle_after: Duration,
}

#[derive(Debug, Default)]
struct TrackerState {
    active: BTreeSet<String>,
    generation: u64,
}

impl SaveQuiescenceTracker {
    pub fn new(idle_after: Duration) -> Self {
        Self {
            inner: Arc::new(Mutex::new(TrackerState::default())),
            state: Arc::new(Mutex::new(SaveQuiescence::Idle)),
            idle_after,
        }
    }

    pub fn current(&self) -> SaveQuiescence {
        self.state.lock().unwrap_or_else(|e| e.into_inner()).clone()
    }

    fn set_state(&self, state: SaveQuiescence) {
        *self.state.lock().unwrap_or_else(|e| e.into_inner()) = state;
    }

    pub fn mark_active(&self, path: impl AsRef<Path>) {
        let paths = {
            let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            inner.generation += 1;
            inner.active.insert(path_key(path.as_ref()));
            inner.active.iter().cloned().collect::<Vec<_>>()
        };
        self.set_state(SaveQuiescence::Active { paths });
    }

    pub fn mark_saved(&self, path: impl AsRef<Path>) {
        let (remaining, generation) = {
            let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            inner.generation += 1;
            inner.active.remove(&path_key(path.as_ref()));
            (
                inner.active.iter().cloned().collect::<Vec<_>>(),
                inner.generation,
            )
        };

        if remaining.is_empty() {
            self.set_state(SaveQuiescence::RecentlySaved);
            self.schedule_idle(generation);
        } else {
            self.set_state(SaveQuiescence::Active { paths: remaining });
        }
    }

    pub fn mark_idle(&self) {
        {
            let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
            inner.generation += 1;
            inner.active.clear();
        }
        self.set_state(SaveQuiescence::Idle);
    }

    fn schedule_idle(&self, generation: u64) {
        let inner = self.inner.clone();
        let state = self.state.clone();
        let delay = self.idle_after;
        tokio::spawn(async move {
            tokio::time::sleep(delay).await;
            let should_idle = {
                let inner = inner.lock().unwrap_or_else(|e| e.into_inner());
                inner.generation == generation && inner.active.is_empty()
            };
            if should_idle {
                *state.lock().unwrap_or_else(|e| e.into_inner()) = SaveQuiescence::Idle;
            }
        });
    }
}

impl Default for SaveQuiescenceTracker {
    fn default() -> Self {
        Self::new(Duration::from_millis(750))
    }
}

fn path_key(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn tracker_reports_active_recently_saved_then_idle() {
        let tracker = SaveQuiescenceTracker::new(Duration::from_millis(10));
        tracker.mark_active("notes/today.md");
        assert_eq!(
            tracker.current(),
            SaveQuiescence::Active {
                paths: vec!["notes/today.md".into()],
            }
        );

        tracker.mark_saved("notes/today.md");
        assert_eq!(tracker.current(), SaveQuiescence::RecentlySaved);

        tokio::time::sleep(Duration::from_millis(30)).await;
        assert_eq!(tracker.current(), SaveQuiescence::Idle);
    }

    #[tokio::test]
    async fn tracker_keeps_active_until_all_paths_saved() {
        let tracker = SaveQuiescenceTracker::new(Duration::from_millis(10));
        tracker.mark_active("a.md");
        tracker.mark_active("b.md");
        tracker.mark_saved("a.md");

        assert_eq!(
            tracker.current(),
            SaveQuiescence::Active {
                paths: vec!["b.md".into()],
            }
        );
    }

    #[tokio::test]
    async fn new_active_save_cancels_pending_idle() {
        let tracker = SaveQuiescenceTracker::new(Duration::from_millis(30));
        tracker.mark_active("a.md");
        tracker.mark_saved("a.md");
        tracker.mark_active("b.md");

        tokio::time::sleep(Duration::from_millis(50)).await;
        assert_eq!(
            tracker.current(),
            SaveQuiescence::Active {
                paths: vec!["b.md".into()],
            }
        );
    }
}
