//! Design Board surface profiles.
//!
//! A surface profile describes what kind of design surface a `.board` file is
//! being used as: website mockup, resume, brochure, whiteboard, diagram, and so
//! on. The profile selects live operator affordances (components, templates,
//! validations, exports, actions) without changing the canonical backing file.
//!
//! Source-of-truth rule: every profile uses plain-text `board_json` source.
//! PNG/PDF/SVG/HTML are generated outputs, never editable canonical state.

use serde::{Deserialize, Serialize};

pub const DESIGN_BOARD_SOURCE_FORMAT: &str = "board_json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DesignBoardKind {
    Website,
    Document,
    Resume,
    Brochure,
    Whiteboard,
    Diagram,
    Dashboard,
    Research,
    Other,
}

impl DesignBoardKind {
    pub const ALL: &'static [Self] = &[
        Self::Website,
        Self::Document,
        Self::Resume,
        Self::Brochure,
        Self::Whiteboard,
        Self::Diagram,
        Self::Dashboard,
        Self::Research,
        Self::Other,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Website => "website",
            Self::Document => "document",
            Self::Resume => "resume",
            Self::Brochure => "brochure",
            Self::Whiteboard => "whiteboard",
            Self::Diagram => "diagram",
            Self::Dashboard => "dashboard",
            Self::Research => "research",
            Self::Other => "other",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DesignBoardSurfaceProfile {
    pub kind: DesignBoardKind,
    pub label: &'static str,
    pub description: &'static str,
    /// Canonical editable backing format. This is intentionally plain text.
    pub source_format: &'static str,
    pub component_categories: &'static [&'static str],
    pub recommended_components: &'static [&'static str],
    pub templates: &'static [&'static str],
    pub page_presets: &'static [&'static str],
    /// Generated outputs only. These must never be treated as source formats.
    pub export_targets: &'static [&'static str],
    pub validation_profiles: &'static [&'static str],
    pub primary_actions: &'static [&'static str],
}

pub fn parse_design_board_kind(value: &str) -> DesignBoardKind {
    match normalize_kind(value).as_str() {
        "website" | "web" | "site" | "landing_page" | "landing" => DesignBoardKind::Website,
        "document" | "doc" | "pdf" | "one_pager" | "one-pager" | "report" => {
            DesignBoardKind::Document
        }
        "resume" | "cv" => DesignBoardKind::Resume,
        "brochure" | "pamphlet" | "flyer" => DesignBoardKind::Brochure,
        "whiteboard" | "board" | "ideation" | "planning" => DesignBoardKind::Whiteboard,
        "diagram" | "architecture" | "flow" | "map" => DesignBoardKind::Diagram,
        "dashboard" | "metrics" | "status" => DesignBoardKind::Dashboard,
        "research" | "source" | "sources" | "evidence" => DesignBoardKind::Research,
        "other" | "generic" | "misc" => DesignBoardKind::Other,
        _ => DesignBoardKind::Other,
    }
}

pub fn surface_profile(kind: DesignBoardKind) -> &'static DesignBoardSurfaceProfile {
    list_surface_profiles()
        .iter()
        .find(|profile| profile.kind == kind)
        .unwrap_or(&OTHER_PROFILE)
}

pub fn list_surface_profiles() -> &'static [DesignBoardSurfaceProfile] {
    &PROFILES
}

fn normalize_kind(value: &str) -> String {
    value
        .trim()
        .to_ascii_lowercase()
        .replace([' ', '-'], "_")
}

const PROFILES: [DesignBoardSurfaceProfile; 9] = [
    WEBSITE_PROFILE,
    DOCUMENT_PROFILE,
    RESUME_PROFILE,
    BROCHURE_PROFILE,
    WHITEBOARD_PROFILE,
    DIAGRAM_PROFILE,
    DASHBOARD_PROFILE,
    RESEARCH_PROFILE,
    OTHER_PROFILE,
];

const WEBSITE_PROFILE: DesignBoardSurfaceProfile = DesignBoardSurfaceProfile {
    kind: DesignBoardKind::Website,
    label: "Website mockup",
    description: "Landing pages, product pages, and responsive website or app-screen mockups.",
    source_format: DESIGN_BOARD_SOURCE_FORMAT,
    component_categories: &["layout", "typography", "actions", "media", "website"],
    recommended_components: &[
        "Frame",
        "TextBlock",
        "Columns",
        "Stack",
        "ButtonRow",
        "ImagePlaceholder",
        "Navbar",
        "Hero",
        "FeatureGrid",
        "CallToAction",
        "DeviceMockup",
        "FormMock",
    ],
    templates: &["landing-page", "saas-one-pager", "product-page", "app-mockup"],
    page_presets: &["web-desktop", "web-tablet", "web-mobile"],
    export_targets: &["png", "html"],
    validation_profiles: &["responsive_bounds", "cta_hierarchy", "contrast", "missing_media"],
    primary_actions: &["change_theme", "preview_mobile", "export_png", "ask_agent_revision"],
};

const DOCUMENT_PROFILE: DesignBoardSurfaceProfile = DesignBoardSurfaceProfile {
    kind: DesignBoardKind::Document,
    label: "Document / PDF",
    description: "Plain-text-backed one-pagers, reports, handouts, and PDF-oriented documents.",
    source_format: DESIGN_BOARD_SOURCE_FORMAT,
    component_categories: &["layout", "typography", "media", "document"],
    recommended_components: &[
        "Frame",
        "TextBlock",
        "Columns",
        "Stack",
        "ImagePlaceholder",
        "PullQuote",
        "Callout",
        "ReferenceList",
    ],
    templates: &["one-page-report", "a4-handout", "letter-handout"],
    page_presets: &["letter", "a4", "legal"],
    export_targets: &["pdf", "png"],
    validation_profiles: &["print_bounds", "font_size", "page_title", "print_safe_colors"],
    primary_actions: &["change_theme", "render_pdf", "export_png", "ask_agent_revision"],
};

const RESUME_PROFILE: DesignBoardSurfaceProfile = DesignBoardSurfaceProfile {
    kind: DesignBoardKind::Resume,
    label: "Resume / CV",
    description: "Professional resume and CV layouts backed by plain-text board JSON.",
    source_format: DESIGN_BOARD_SOURCE_FORMAT,
    component_categories: &["layout", "typography", "resume"],
    recommended_components: &[
        "Frame",
        "TextBlock",
        "Columns",
        "Stack",
        "ResumeHeader",
        "ExperienceItem",
        "SkillList",
        "Timeline",
        "ContactCard",
    ],
    templates: &["resume-one-page", "resume-technical", "resume-compact"],
    page_presets: &["letter", "a4"],
    export_targets: &["pdf", "png"],
    validation_profiles: &["print_bounds", "font_size", "contact_info", "date_consistency"],
    primary_actions: &["change_theme", "render_pdf", "export_png", "ask_agent_revision"],
};

const BROCHURE_PROFILE: DesignBoardSurfaceProfile = DesignBoardSurfaceProfile {
    kind: DesignBoardKind::Brochure,
    label: "Brochure / pamphlet",
    description: "Marketing flyers, product pamphlets, and brochure-like layouts.",
    source_format: DESIGN_BOARD_SOURCE_FORMAT,
    component_categories: &["layout", "typography", "media", "marketing"],
    recommended_components: &[
        "Frame",
        "TextBlock",
        "Columns",
        "Stack",
        "ButtonRow",
        "ImagePlaceholder",
        "FeatureGrid",
        "CallToAction",
        "BrochurePanel",
        "PullQuote",
    ],
    templates: &["product-flyer", "brochure-trifold", "marketing-one-pager"],
    page_presets: &["letter", "a4", "brochure-trifold"],
    export_targets: &["pdf", "png"],
    validation_profiles: &["print_bounds", "fold_guides", "missing_media", "contact_info"],
    primary_actions: &["change_theme", "render_pdf", "export_png", "ask_agent_revision"],
};

const WHITEBOARD_PROFILE: DesignBoardSurfaceProfile = DesignBoardSurfaceProfile {
    kind: DesignBoardKind::Whiteboard,
    label: "Whiteboard",
    description: "Personal ideation, planning, loose thinking, swimlanes, and project sketching.",
    source_format: DESIGN_BOARD_SOURCE_FORMAT,
    component_categories: &["layout", "typography", "whiteboard", "diagram"],
    recommended_components: &[
        "Frame",
        "TextBlock",
        "Stack",
        "StickyNote",
        "Checklist",
        "Swimlane",
        "ProcessSteps",
        "MindMapLite",
    ],
    templates: &["personal-whiteboard", "project-plan", "brainstorm"],
    page_presets: &["freeform", "slide", "wide"],
    export_targets: &["png", "markdown"],
    validation_profiles: &["readability", "empty_notes"],
    primary_actions: &["change_theme", "export_png", "summarize_markdown", "ask_agent_revision"],
};

const DIAGRAM_PROFILE: DesignBoardSurfaceProfile = DesignBoardSurfaceProfile {
    kind: DesignBoardKind::Diagram,
    label: "Diagram",
    description: "Architecture maps, process diagrams, system flows, and text-authored diagram panels.",
    source_format: DESIGN_BOARD_SOURCE_FORMAT,
    component_categories: &["layout", "typography", "diagram"],
    recommended_components: &[
        "Frame",
        "TextBlock",
        "ProcessSteps",
        "FlowDiagram",
        "ArchitectureMap",
        "DiagramPanel",
    ],
    templates: &["architecture-overview", "process-flow", "system-map"],
    page_presets: &["slide", "wide", "a4"],
    export_targets: &["svg", "png", "pdf"],
    validation_profiles: &["diagram_connectivity", "diagram_bounds", "svg_sanitization"],
    primary_actions: &["change_theme", "export_svg", "export_png", "ask_agent_revision"],
};

const DASHBOARD_PROFILE: DesignBoardSurfaceProfile = DesignBoardSurfaceProfile {
    kind: DesignBoardKind::Dashboard,
    label: "Dashboard",
    description: "Metrics, status summaries, comparisons, and operational overview boards.",
    source_format: DESIGN_BOARD_SOURCE_FORMAT,
    component_categories: &["layout", "typography", "data", "actions"],
    recommended_components: &[
        "Frame",
        "TextBlock",
        "Columns",
        "Stack",
        "Panel",
        "MetricCard",
        "StatGrid",
        "ComparisonTable",
        "ProgressCard",
    ],
    templates: &["project-dashboard", "status-overview", "metrics-review"],
    page_presets: &["web-desktop", "slide", "wide"],
    export_targets: &["png", "pdf"],
    validation_profiles: &["metric_labels", "tone_consistency", "stale_data"],
    primary_actions: &["change_theme", "export_png", "render_pdf", "ask_agent_revision"],
};

const RESEARCH_PROFILE: DesignBoardSurfaceProfile = DesignBoardSurfaceProfile {
    kind: DesignBoardKind::Research,
    label: "Research board",
    description: "Source-backed research synthesis boards with claims, evidence, tasks, and citations.",
    source_format: DESIGN_BOARD_SOURCE_FORMAT,
    component_categories: &["layout", "typography", "research", "data", "whiteboard"],
    recommended_components: &[
        "Frame",
        "TextBlock",
        "Columns",
        "Stack",
        "Panel",
        "SourceCard",
        "EvidenceCard",
        "TaskCard",
        "ClaimCard",
        "ComparisonTable",
    ],
    templates: &["source-review", "evidence-map", "research-dashboard"],
    page_presets: &["web-desktop", "wide", "a4"],
    export_targets: &["png", "pdf", "markdown"],
    validation_profiles: &["source_refs", "stale_projection", "claim_evidence", "citation_keys"],
    primary_actions: &["refresh_sources", "export_png", "summarize_markdown", "ask_agent_revision"],
};

const OTHER_PROFILE: DesignBoardSurfaceProfile = DesignBoardSurfaceProfile {
    kind: DesignBoardKind::Other,
    label: "Generic design board",
    description: "Fallback visual composition surface with core components and generic exports.",
    source_format: DESIGN_BOARD_SOURCE_FORMAT,
    component_categories: &["layout", "typography", "media", "actions"],
    recommended_components: &[
        "Frame",
        "TextBlock",
        "Columns",
        "Stack",
        "Panel",
        "ButtonRow",
        "ImagePlaceholder",
    ],
    templates: &["blank", "basic-layout"],
    page_presets: &["freeform", "web-desktop", "letter", "a4"],
    export_targets: &["png", "pdf"],
    validation_profiles: &["structure", "readability", "contrast"],
    primary_actions: &["change_theme", "export_png", "ask_agent_revision"],
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    const BINARY_DESIGN_SOURCE_FORMATS: &[&str] = &["pdf", "png", "jpg", "jpeg", "webp", "fig", "psd", "sketch"];

    #[test]
    fn parses_all_canonical_kind_strings() {
        for kind in DesignBoardKind::ALL {
            assert_eq!(parse_design_board_kind(kind.as_str()), *kind);
        }
    }

    #[test]
    fn parses_common_aliases() {
        let cases = [
            ("web", DesignBoardKind::Website),
            ("landing page", DesignBoardKind::Website),
            ("PDF", DesignBoardKind::Document),
            ("one-pager", DesignBoardKind::Document),
            ("cv", DesignBoardKind::Resume),
            ("pamphlet", DesignBoardKind::Brochure),
            ("ideation", DesignBoardKind::Whiteboard),
            ("architecture", DesignBoardKind::Diagram),
            ("metrics", DesignBoardKind::Dashboard),
            ("evidence", DesignBoardKind::Research),
            ("misc", DesignBoardKind::Other),
        ];
        for (input, expected) in cases {
            assert_eq!(parse_design_board_kind(input), expected, "input={input}");
        }
    }

    #[test]
    fn unknown_kind_falls_back_to_other() {
        assert_eq!(parse_design_board_kind("banana"), DesignBoardKind::Other);
        assert_eq!(parse_design_board_kind(""), DesignBoardKind::Other);
        assert_eq!(parse_design_board_kind("   "), DesignBoardKind::Other);
    }

    #[test]
    fn profile_exists_for_every_kind() {
        let profile_kinds: HashSet<_> = list_surface_profiles().iter().map(|p| p.kind).collect();
        for kind in DesignBoardKind::ALL {
            assert!(profile_kinds.contains(kind), "missing profile for {kind:?}");
        }
        assert_eq!(profile_kinds.len(), DesignBoardKind::ALL.len());
    }

    #[test]
    fn surface_profile_returns_matching_kind() {
        for kind in DesignBoardKind::ALL {
            assert_eq!(surface_profile(*kind).kind, *kind);
        }
    }

    #[test]
    fn every_profile_uses_plain_text_board_json_source() {
        for profile in list_surface_profiles() {
            assert_eq!(profile.source_format, DESIGN_BOARD_SOURCE_FORMAT, "{}", profile.label);
            assert!(!BINARY_DESIGN_SOURCE_FORMATS.contains(&profile.source_format));
        }
    }

    #[test]
    fn export_targets_do_not_claim_to_be_source_formats() {
        for profile in list_surface_profiles() {
            assert!(!profile.export_targets.is_empty(), "{} missing exports", profile.label);
            for target in profile.export_targets {
                assert_ne!(*target, profile.source_format, "{} exports source format", profile.label);
            }
        }
    }

    #[test]
    fn profiles_have_operator_affordance_fields() {
        for profile in list_surface_profiles() {
            assert!(!profile.label.trim().is_empty());
            assert!(!profile.description.trim().is_empty());
            assert!(!profile.component_categories.is_empty(), "{} missing categories", profile.label);
            assert!(!profile.recommended_components.is_empty(), "{} missing components", profile.label);
            assert!(!profile.templates.is_empty(), "{} missing templates", profile.label);
            assert!(!profile.page_presets.is_empty(), "{} missing page presets", profile.label);
            assert!(!profile.validation_profiles.is_empty(), "{} missing validations", profile.label);
            assert!(!profile.primary_actions.is_empty(), "{} missing actions", profile.label);
        }
    }

    #[test]
    fn implemented_foundation_components_are_recommended_somewhere() {
        let all_recommended: HashSet<_> = list_surface_profiles()
            .iter()
            .flat_map(|profile| profile.recommended_components.iter().copied())
            .collect();
        for component in ["Frame", "TextBlock", "Columns", "Stack", "ButtonRow", "ImagePlaceholder", "Panel"] {
            assert!(all_recommended.contains(component), "{component} is not recommended by any profile");
        }
    }

    #[test]
    fn each_profile_includes_at_least_one_foundation_component() {
        let foundation = ["Frame", "TextBlock", "Columns", "Stack", "ButtonRow", "ImagePlaceholder", "Panel"];
        for profile in list_surface_profiles() {
            assert!(
                profile
                    .recommended_components
                    .iter()
                    .any(|component| foundation.contains(component)),
                "{} has no foundation component",
                profile.label
            );
        }
    }

    #[test]
    fn print_or_document_profiles_can_render_pdf_but_still_use_board_json() {
        for kind in [DesignBoardKind::Document, DesignBoardKind::Resume, DesignBoardKind::Brochure] {
            let profile = surface_profile(kind);
            assert!(profile.export_targets.contains(&"pdf"));
            assert_eq!(profile.source_format, "board_json");
        }
    }

    #[test]
    fn website_profile_does_not_treat_html_as_source() {
        let profile = surface_profile(DesignBoardKind::Website);
        assert!(profile.export_targets.contains(&"html"));
        assert_eq!(profile.source_format, "board_json");
    }

    #[test]
    fn profile_kind_strings_are_unique() {
        let mut seen = HashSet::new();
        for kind in DesignBoardKind::ALL {
            assert!(seen.insert(kind.as_str()), "duplicate kind string: {}", kind.as_str());
        }
    }

    #[test]
    fn profile_array_has_no_duplicate_kinds() {
        let mut seen = HashSet::new();
        for profile in list_surface_profiles() {
            assert!(seen.insert(profile.kind), "duplicate profile kind: {:?}", profile.kind);
        }
    }
}
