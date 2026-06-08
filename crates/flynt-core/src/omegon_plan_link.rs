//! Helpers for linking Flynt tasks to Omegon ACP plan/task projections.
//!
//! The current Omegon ACP plan/task surface is projection-first. Until Omegon
//! guarantees stable IDs, durable binds, revisions, and mutation semantics,
//! Flynt stores links as local external references and treats upstream binds as
//! best-effort session hints.

use serde::{Deserialize, Serialize};

pub const OMEGON_PLAN_REF_PREFIX: &str = "omegon-plan:";
pub const OMEGON_PROMOTION_DRAFT_REF_PREFIX: &str = "omegon-promotion-draft:";
pub const OMEGON_BINDING_STATE_REF_PREFIX: &str = "omegon-binding:";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OmegonPlanTaskLink {
    pub plan_id: String,
    pub task_id: String,
    pub label: Option<String>,
    pub revision: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OmegonBindingState {
    pub system: String,
    pub external_task_id: String,
    pub durability: String,
    pub status: String,
    pub task_id: Option<String>,
    pub stable_id: Option<String>,
    pub revision: Option<String>,
    pub code: Option<String>,
    pub message: Option<String>,
}

impl OmegonBindingState {
    pub fn session_import(
        external_task_id: impl Into<String>,
        message: Option<impl Into<String>>,
    ) -> Self {
        Self {
            system: "flynt".to_string(),
            external_task_id: external_task_id.into(),
            durability: "session".to_string(),
            status: "session_bound".to_string(),
            task_id: None,
            stable_id: None,
            revision: None,
            code: None,
            message: message.map(Into::into),
        }
    }

    pub fn to_external_ref(&self) -> String {
        format!(
            "{OMEGON_BINDING_STATE_REF_PREFIX}{}",
            serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
        )
    }

    pub fn from_external_ref(value: &str) -> Option<Self> {
        let payload = value.strip_prefix(OMEGON_BINDING_STATE_REF_PREFIX)?;
        let state: Self = serde_json::from_str(payload).ok()?;
        if state.external_task_id.trim().is_empty() || state.durability.trim().is_empty() {
            return None;
        }
        Some(state)
    }

    pub fn is_repo_bound(&self) -> bool {
        self.status == "repo_bound" && self.durability == "repo"
    }

    pub fn is_session_bound(&self) -> bool {
        self.status == "session_bound" && self.durability == "session"
    }

    pub fn is_conflict(&self) -> bool {
        matches!(self.status.as_str(), "stale" | "conflict")
            || matches!(self.code.as_deref(), Some("stale_revision" | "conflict"))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OmegonPromotionDraft {
    pub kind: String,
    pub status: String,
    pub system: String,
    pub external_task_id: String,
    pub created_at: String,
    pub target_hint: OmegonPromotionTargetHint,
    pub original: OmegonPromotionOriginalTask,
    pub review: OmegonPromotionReview,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub struct OmegonPromotionTargetHint {
    pub kind: String,
    pub change: Option<String>,
    pub node_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OmegonPromotionOriginalTask {
    pub title: String,
    pub body: String,
    pub board_id: String,
    pub column: String,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OmegonPromotionReview {
    pub required: bool,
    pub reason: String,
}

impl OmegonPromotionDraft {
    pub fn new(
        external_task_id: impl Into<String>,
        created_at: impl Into<String>,
        original: OmegonPromotionOriginalTask,
        reason: impl Into<String>,
    ) -> Self {
        Self {
            kind: "omegon_external_task_promotion_draft".to_string(),
            status: "pending_review".to_string(),
            system: "flynt".to_string(),
            external_task_id: external_task_id.into(),
            created_at: created_at.into(),
            target_hint: OmegonPromotionTargetHint {
                kind: "unknown".to_string(),
                ..Default::default()
            },
            original,
            review: OmegonPromotionReview {
                required: true,
                reason: reason.into(),
            },
        }
    }

    pub fn to_external_ref(&self) -> String {
        format!(
            "{OMEGON_PROMOTION_DRAFT_REF_PREFIX}{}",
            serde_json::to_string(self).unwrap_or_else(|_| "{}".to_string())
        )
    }

    pub fn from_external_ref(value: &str) -> Option<Self> {
        let payload = value.strip_prefix(OMEGON_PROMOTION_DRAFT_REF_PREFIX)?;
        let draft: Self = serde_json::from_str(payload).ok()?;
        if draft.kind != "omegon_external_task_promotion_draft"
            || draft.external_task_id.trim().is_empty()
        {
            return None;
        }
        Some(draft)
    }
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

pub fn find_omegon_binding_states<'a>(
    refs: impl IntoIterator<Item = &'a String>,
) -> Vec<OmegonBindingState> {
    refs.into_iter()
        .filter_map(|value| OmegonBindingState::from_external_ref(value))
        .collect()
}

pub fn upsert_omegon_binding_state(refs: &mut Vec<String>, state: OmegonBindingState) {
    refs.retain(|value| {
        OmegonBindingState::from_external_ref(value)
            .map(|existing| existing.external_task_id != state.external_task_id)
            .unwrap_or(true)
    });
    refs.push(state.to_external_ref());
}

pub fn find_omegon_promotion_drafts<'a>(
    refs: impl IntoIterator<Item = &'a String>,
) -> Vec<OmegonPromotionDraft> {
    refs.into_iter()
        .filter_map(|value| OmegonPromotionDraft::from_external_ref(value))
        .collect()
}

pub fn upsert_omegon_promotion_draft(refs: &mut Vec<String>, draft: OmegonPromotionDraft) {
    refs.retain(|value| {
        OmegonPromotionDraft::from_external_ref(value)
            .map(|existing| existing.external_task_id != draft.external_task_id)
            .unwrap_or(true)
    });
    refs.push(draft.to_external_ref());
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
    fn binding_state_round_trips_and_classifies() {
        let state = OmegonBindingState::session_import("flynt-task-1", Some("review required"));
        let encoded = state.to_external_ref();
        assert!(encoded.starts_with(OMEGON_BINDING_STATE_REF_PREFIX));
        let decoded = OmegonBindingState::from_external_ref(&encoded).unwrap();
        assert!(decoded.is_session_bound());
        assert!(!decoded.is_repo_bound());
        assert!(!decoded.is_conflict());
    }

    #[test]
    fn upsert_replaces_existing_binding_state_for_task() {
        let mut refs = vec!["https://example.com".to_string()];
        upsert_omegon_binding_state(
            &mut refs,
            OmegonBindingState::session_import("task", Some("first")),
        );
        upsert_omegon_binding_state(
            &mut refs,
            OmegonBindingState::session_import("task", Some("second")),
        );
        let states = find_omegon_binding_states(&refs);
        assert_eq!(refs.len(), 2);
        assert_eq!(states.len(), 1);
        assert_eq!(states[0].message.as_deref(), Some("second"));
    }

    #[test]
    fn promotion_draft_round_trips_external_ref() {
        let draft = OmegonPromotionDraft::new(
            "flynt-task-1",
            "2026-06-07T00:00:00Z",
            OmegonPromotionOriginalTask {
                title: "Promote me".into(),
                body: "Body".into(),
                board_id: "board-1".into(),
                column: "Backlog".into(),
                tags: vec!["x".into()],
            },
            "Omegon was unavailable",
        );

        let encoded = draft.to_external_ref();
        assert!(encoded.starts_with(OMEGON_PROMOTION_DRAFT_REF_PREFIX));
        let decoded = OmegonPromotionDraft::from_external_ref(&encoded).unwrap();
        assert_eq!(decoded.kind, "omegon_external_task_promotion_draft");
        assert_eq!(decoded.status, "pending_review");
        assert_eq!(decoded.external_task_id, "flynt-task-1");
        assert!(decoded.review.required);
    }

    #[test]
    fn upsert_replaces_existing_promotion_draft_for_task() {
        let original = OmegonPromotionOriginalTask {
            title: "one".into(),
            body: String::new(),
            board_id: "board".into(),
            column: "Backlog".into(),
            tags: Vec::new(),
        };
        let mut refs = vec!["https://example.com".to_string()];
        upsert_omegon_promotion_draft(
            &mut refs,
            OmegonPromotionDraft::new("task", "t1", original.clone(), "first"),
        );
        upsert_omegon_promotion_draft(
            &mut refs,
            OmegonPromotionDraft::new("task", "t2", original, "second"),
        );

        let drafts = find_omegon_promotion_drafts(&refs);
        assert_eq!(refs.len(), 2);
        assert_eq!(drafts.len(), 1);
        assert_eq!(drafts[0].created_at, "t2");
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
