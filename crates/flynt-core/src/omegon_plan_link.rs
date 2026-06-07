//! Helpers for linking Flynt tasks to Omegon ACP plan/task projections.
//!
//! The current Omegon ACP plan/task surface is projection-first. Until Omegon
//! guarantees stable IDs, durable binds, revisions, and mutation semantics,
//! Flynt stores links as local external references and treats upstream binds as
//! best-effort session hints.

use serde::{Deserialize, Serialize};

pub const OMEGON_PLAN_REF_PREFIX: &str = "omegon-plan:";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OmegonPlanTaskLink {
    pub plan_id: String,
    pub task_id: String,
    pub label: Option<String>,
    pub revision: Option<String>,
}

impl OmegonPlanTaskLink {
    pub fn new(plan_id: impl Into<String>, task_id: impl Into<String>) -> Self {
        Self {
            plan_id: plan_id.into(),
            task_id: task_id.into(),
            label: None,
            revision: None,
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn with_revision(mut self, revision: impl Into<String>) -> Self {
        self.revision = Some(revision.into());
        self
    }

    /// Encode as a single external-ref string suitable for `Task.external_refs`.
    ///
    /// The payload is JSON after the `omegon-plan:` prefix so IDs can contain
    /// punctuation such as `openspec:foo:group:bar:1.2` without inventing a URL
    /// escaping contract. Consumers must treat this as a Flynt-local durable
    /// mapping, not proof that Omegon persisted a reciprocal binding.
    pub fn to_external_ref(&self) -> String {
        format!(
            "{OMEGON_PLAN_REF_PREFIX}{}",
            serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
        )
    }

    pub fn from_external_ref(value: &str) -> Option<Self> {
        let payload = value.strip_prefix(OMEGON_PLAN_REF_PREFIX)?;
        let link: Self = serde_json::from_str(payload).ok()?;
        if link.plan_id.trim().is_empty() || link.task_id.trim().is_empty() {
            return None;
        }
        Some(link)
    }
}

pub fn find_omegon_plan_task_links<'a>(
    refs: impl IntoIterator<Item = &'a String>,
) -> Vec<OmegonPlanTaskLink> {
    refs.into_iter()
        .filter_map(|value| OmegonPlanTaskLink::from_external_ref(value))
        .collect()
}

pub fn upsert_omegon_plan_task_link(refs: &mut Vec<String>, link: OmegonPlanTaskLink) {
    refs.retain(|value| {
        OmegonPlanTaskLink::from_external_ref(value)
            .map(|existing| existing.task_id != link.task_id || existing.plan_id != link.plan_id)
            .unwrap_or(true)
    });
    refs.push(link.to_external_ref());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_external_ref() {
        let link = OmegonPlanTaskLink::new("openspec:sync", "openspec:sync:group:One:1.2")
            .with_label("Validate sync")
            .with_revision("sha256:abc");

        let encoded = link.to_external_ref();
        assert!(encoded.starts_with(OMEGON_PLAN_REF_PREFIX));
        assert_eq!(OmegonPlanTaskLink::from_external_ref(&encoded), Some(link));
    }

    #[test]
    fn rejects_malformed_or_empty_links() {
        assert_eq!(
            OmegonPlanTaskLink::from_external_ref("https://example.com"),
            None
        );
        assert_eq!(
            OmegonPlanTaskLink::from_external_ref("omegon-plan:not-json"),
            None
        );
        assert_eq!(
            OmegonPlanTaskLink::from_external_ref(
                "omegon-plan:{\"plan_id\":\"\",\"task_id\":\"x\"}"
            ),
            None
        );
    }

    #[test]
    fn upsert_replaces_same_plan_task_link_only() {
        let mut refs = vec!["https://example.com".to_string()];
        upsert_omegon_plan_task_link(
            &mut refs,
            OmegonPlanTaskLink::new("plan", "task").with_label("old"),
        );
        upsert_omegon_plan_task_link(
            &mut refs,
            OmegonPlanTaskLink::new("plan", "task").with_label("new"),
        );

        assert_eq!(refs.len(), 2);
        let links = find_omegon_plan_task_links(&refs);
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].label.as_deref(), Some("new"));
    }
}
