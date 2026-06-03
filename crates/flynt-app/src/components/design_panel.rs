use crate::{
    bootstrap::AppContext,
    state::{Route, TabState},
};
use dioxus::prelude::*;
use flynt_core::{models::DocumentMeta, store::ProjectStore};
use std::{collections::BTreeMap, path::Path};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DesignPanelTab {
    Files,
    Kinds,
    Components,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComponentGroup {
    pub category: String,
    pub components: Vec<ComponentSummary>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComponentSummary {
    pub name: String,
    pub description: String,
    pub variants: Vec<String>,
}

pub fn grouped_component_summaries() -> Vec<ComponentGroup> {
    let mut by_category = BTreeMap::<String, Vec<ComponentSummary>>::new();
    for component in flynt_core::design_components::list_components() {
        by_category
            .entry(component.category.to_string())
            .or_default()
            .push(ComponentSummary {
                name: component.name.to_string(),
                description: component.description.to_string(),
                variants: component
                    .variants
                    .iter()
                    .map(|variant| variant.to_string())
                    .collect(),
            });
    }
    by_category
        .into_iter()
        .map(|(category, mut components)| {
            components.sort_by(|a, b| a.name.cmp(&b.name));
            ComponentGroup {
                category,
                components,
            }
        })
        .collect()
}

#[component]
pub fn DesignPanel(
    docs: Vec<DocumentMeta>,
    mut refresh: Signal<u64>,
    mut active_route: Signal<Route>,
) -> Element {
    let profiles = flynt_core::design_surfaces::list_surface_profiles();
    let groups = grouped_component_summaries();
    let mut tab = use_signal(|| DesignPanelTab::Files);

    rsx! {
        div { class: "design-panel",
            section { class: "design-panel-section design-panel-intro",
                div { class: "design-panel-heading", "Design" }
                p { class: "design-panel-hint",
                    "Create and manage visual surfaces: boards, drawings, flows, templates, and components."
                }
            }

            div { class: "design-tab-bar",
                button {
                    class: if *tab.read() == DesignPanelTab::Files { "design-tab active" } else { "design-tab" },
                    onclick: move |_| tab.set(DesignPanelTab::Files),
                    "Files"
                }
                button {
                    class: if *tab.read() == DesignPanelTab::Kinds { "design-tab active" } else { "design-tab" },
                    onclick: move |_| tab.set(DesignPanelTab::Kinds),
                    "Kinds"
                }
                button {
                    class: if *tab.read() == DesignPanelTab::Components { "design-tab active" } else { "design-tab" },
                    onclick: move |_| tab.set(DesignPanelTab::Components),
                    "Components"
                }
            }

            match *tab.read() {
                DesignPanelTab::Files => rsx! {
                    DesignFilesTab { docs, refresh, active_route }
                },
                DesignPanelTab::Kinds => rsx! {
                    section { class: "design-panel-section",
                        div { class: "design-panel-subheading", "Surface kinds" }
                        div { class: "design-kind-list",
                            for profile in profiles.iter() {
                                div { key: "{profile.kind.as_str()}", class: "design-kind-row",
                                    div { class: "design-kind-main",
                                        span { class: "design-kind-title", "{profile.label}" }
                                        span { class: "design-kind-description", "{profile.description}" }
                                    }
                                    span { class: "design-kind-exports", "{profile.export_targets.join(\" / \")}" }
                                }
                            }
                        }
                    }
                },
                DesignPanelTab::Components => rsx! {
                    section { class: "design-panel-section",
                        div { class: "design-panel-subheading", "Components" }
                        for group in groups.iter() {
                            div { key: "{group.category}", class: "design-component-group",
                                div { class: "design-component-category", "{group.category}" }
                                for component in group.components.iter() {
                                    div { key: "{component.name}", class: "design-component-row",
                                        div { class: "design-component-row-main",
                                            span { class: "design-component-title", "{component.name}" }
                                            span { class: "design-component-description", "{component.description}" }
                                        }
                                        span { class: "design-component-count", "{component.variants.len()}" }
                                    }
                                }
                            }
                        }
                    }
                },
            }
        }
    }
}

#[component]
fn DesignFilesTab(
    docs: Vec<DocumentMeta>,
    mut refresh: Signal<u64>,
    mut active_route: Signal<Route>,
) -> Element {
    let artifacts = design_artifacts(&docs);
    let ctx = use_context::<AppContext>();
    let tab_state = use_context::<Signal<TabState>>();

    rsx! {
        section { class: "design-panel-section",
            div { class: "design-panel-subheading", "Files" }
            div { class: "design-file-actions",
                button {
                    class: "design-action-button primary",
                    onclick: move |_| create_design_board(ctx.clone(), tab_state, active_route, refresh),
                    "New Design Board"
                }
                button {
                    class: "design-action-button",
                    onclick: move |_| create_drawing(ctx.clone(), tab_state, active_route, refresh),
                    "New Drawing"
                }
                button {
                    class: "design-action-button",
                    onclick: move |_| create_flow(ctx.clone(), tab_state, active_route, refresh),
                    "New Flow"
                }
            }
            if artifacts.is_empty() {
                div { class: "design-empty-state",
                    div { class: "design-empty-title", "No visual surfaces yet" }
                    p { "Create a board, drawing, or flow to start building the design inventory." }
                }
            } else {
                div { class: "design-artifact-list",
                    for artifact in artifacts {
                        DesignArtifactRow {
                            artifact,
                            refresh,
                            active_route,
                        }
                    }
                }
            }
            div { class: "design-panel-subheading", "Quick orientation" }
            div { class: "design-quick-list",
                div { class: "design-quick-row",
                    span { "Canonical source" }
                    strong { "wrappers + backing files" }
                }
                div { class: "design-quick-row",
                    span { "Default workflow" }
                    strong { "create → open → revise" }
                }
                div { class: "design-quick-row",
                    span { "Managed here" }
                    strong { "boards, drawings, flows" }
                }
            }
        }
    }
}

#[derive(Clone, PartialEq)]
struct DesignArtifact {
    doc: DocumentMeta,
    kind: DesignArtifactKind,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DesignArtifactKind {
    Board,
    Drawing,
    Flow,
}

impl DesignArtifactKind {
    fn label(self) -> &'static str {
        match self {
            Self::Board => "Board",
            Self::Drawing => "Drawing",
            Self::Flow => "Flow",
        }
    }
}

fn create_design_board(
    ctx: AppContext,
    mut tab_state: Signal<TabState>,
    mut active_route: Signal<Route>,
    mut refresh: Signal<u64>,
) {
    spawn(async move {
        let project = ctx.project();
        let ts_suffix = chrono::Local::now().format("%Y%m%d-%H%M%S%3f").to_string();
        let name = format!("DesignBoard {ts_suffix}");
        if let Ok(md_path) = crate::views::design_board::create_design_board(&project.root, &name) {
            let _ = project.index_file(&project.root.join(&md_path));
            let _ =
                ctx.project_events()
                    .send(flynt_store::watcher::ProjectChangeEvent::FileCreated(
                        project.root.join(&md_path),
                    ));
            let slug = name.to_lowercase();
            if let Ok(Some(doc)) = project.store.find_document_by_slug(&slug) {
                tab_state.write().open(doc.id, name);
            }
            *active_route.write() = Route::Notes;
            refresh += 1;
        }
    });
}

fn create_drawing(
    ctx: AppContext,
    mut tab_state: Signal<TabState>,
    mut active_route: Signal<Route>,
    mut refresh: Signal<u64>,
) {
    spawn(async move {
        let project = ctx.project();
        let ts_suffix = chrono::Local::now().format("%Y%m%d-%H%M%S%3f").to_string();
        let name = format!("Drawing {ts_suffix}");
        if crate::views::excalidraw::create_drawing(&project.root, &name).is_ok() {
            let _ = project.reindex();
            let slug = name.to_lowercase();
            if let Ok(Some(doc)) = project.store.find_document_by_slug(&slug) {
                tab_state.write().open(doc.id, name);
            }
            *active_route.write() = Route::Notes;
            refresh += 1;
        }
    });
}

fn create_flow(
    ctx: AppContext,
    mut tab_state: Signal<TabState>,
    mut active_route: Signal<Route>,
    mut refresh: Signal<u64>,
) {
    spawn(async move {
        let project = ctx.project();
        let ts_suffix = chrono::Local::now().format("%Y%m%d-%H%M%S%3f").to_string();
        let name = format!("Flow {ts_suffix}");
        let rel = std::path::PathBuf::from("flows").join(format!("{name}.flow"));
        let abs = project.root.join(&rel);
        let flow = flynt_flow::Flow {
            meta: flynt_flow::FlowMeta {
                title: Some(name.clone()),
                description: None,
            },
            ..Default::default()
        };
        if flynt_flow::save_flow(&abs, &flow, None).is_ok() {
            let _ = project.index_file(&abs);
            let _ = ctx
                .project_events()
                .send(flynt_store::watcher::ProjectChangeEvent::FileCreated(abs));
            let slug = name.to_lowercase();
            if let Ok(Some(doc)) = project.store.find_document_by_slug(&slug) {
                tab_state.write().open(doc.id, name);
            }
            *active_route.write() = Route::Notes;
            refresh += 1;
        }
    });
}

fn design_artifacts(docs: &[DocumentMeta]) -> Vec<DesignArtifact> {
    let mut artifacts: Vec<_> = docs
        .iter()
        .filter_map(|doc| {
            design_artifact_kind(&doc.path).map(|kind| DesignArtifact {
                doc: doc.clone(),
                kind,
            })
        })
        .collect();
    artifacts.sort_by(|a, b| a.doc.path.cmp(&b.doc.path));
    artifacts
}

fn design_artifact_kind(path: &Path) -> Option<DesignArtifactKind> {
    let path_text = path.to_string_lossy();
    if path_text.starts_with("boards/") && path.extension().is_some_and(|ext| ext == "md") {
        Some(DesignArtifactKind::Board)
    } else if path_text.starts_with("drawings/") && path.extension().is_some_and(|ext| ext == "md")
    {
        Some(DesignArtifactKind::Drawing)
    } else if path.extension().is_some_and(|ext| ext == "flow") {
        Some(DesignArtifactKind::Flow)
    } else {
        None
    }
}

fn artifact_title(doc: &DocumentMeta) -> String {
    doc.path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .map_or_else(
            || doc.path.to_string_lossy().to_string(),
            ToString::to_string,
        )
}

fn open_artifact(
    artifact: &DesignArtifact,
    tab_state: &mut Signal<TabState>,
    active_route: &mut Signal<Route>,
) {
    tab_state
        .write()
        .open(artifact.doc.id.clone(), artifact_title(&artifact.doc));
    *active_route.write() = Route::Notes;
}

#[component]
fn DesignArtifactRow(
    artifact: DesignArtifact,
    mut refresh: Signal<u64>,
    mut active_route: Signal<Route>,
) -> Element {
    let ctx = use_context::<AppContext>();
    let mut tab_state = use_context::<Signal<TabState>>();
    let mut menu_pos: Signal<Option<(f64, f64)>> = use_signal(|| None);
    let title = artifact_title(&artifact.doc);
    let path = artifact.doc.path.to_string_lossy().to_string();
    let kind = artifact.kind.label();

    rsx! {
        div { class: "design-artifact-row-wrap",
            button {
                class: "design-artifact-row",
                title: "{path}",
                onclick: {
                    let artifact = artifact.clone();
                    move |_| {
                        open_artifact(&artifact, &mut tab_state, &mut active_route);
                    }
                },
                oncontextmenu: move |event| {
                    event.prevent_default();
                    let coords = event.client_coordinates();
                    menu_pos.set(Some((coords.x, coords.y)));
                },
                span { class: "design-artifact-kind", "{kind}" }
                span { class: "design-artifact-main",
                    span { class: "design-artifact-title", "{title}" }
                    span { class: "design-artifact-path", "{path}" }
                }
            }
            if let Some((x, y)) = *menu_pos.read() {
                crate::components::ContextMenu {
                    x, y,
                    items: vec![
                        crate::components::ContextMenuItem::new("open", "Open"),
                        crate::components::ContextMenuItem::new("open-tab", "Open in New Tab"),
                        crate::components::ContextMenuItem::new("reveal", if cfg!(target_os = "macos") { "Reveal in Finder" } else { "Open in File Manager" }),
                        crate::components::ContextMenuItem::danger("delete", "Delete").sep(),
                    ],
                    on_close: move |_| menu_pos.set(None),
                    on_select: {
                        let artifact = artifact.clone();
                        move |action: String| {
                        menu_pos.set(None);
                        match action.as_str() {
                            "open" | "open-tab" => {
                                open_artifact(&artifact, &mut tab_state, &mut active_route);
                            }
                            "reveal" => {
                                let abs = ctx.project().root.join(&artifact.doc.path);
                                #[cfg(target_os = "macos")]
                                { let _ = std::process::Command::new("open").arg("-R").arg(&abs).spawn(); }
                                #[cfg(target_os = "linux")]
                                { if let Some(dir) = abs.parent() { let _ = std::process::Command::new("xdg-open").arg(dir).spawn(); } }
                            }
                            "delete" => {
                                let project = ctx.project();
                                let rel = artifact.doc.path.clone();
                                spawn(async move {
                                    let abs = project.root.join(&rel);
                                    let _ = tokio::fs::remove_file(&abs).await;
                                    let _ = project.reindex();
                                    refresh += 1;
                                });
                            }
                            _ => {}
                        }
                        }
                    },
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn groups_components_by_category() {
        let groups = grouped_component_summaries();
        assert!(groups.iter().any(|group| group.category == "layout"));
        assert!(groups.iter().any(|group| group.category == "typography"));
        assert!(groups.iter().any(|group| group.category == "actions"));
        assert!(groups.iter().any(|group| group.category == "media"));
    }

    #[test]
    fn groups_include_foundation_components() {
        let all: Vec<_> = grouped_component_summaries()
            .into_iter()
            .flat_map(|group| group.components.into_iter().map(|component| component.name))
            .collect();
        for expected in [
            "Frame",
            "TextBlock",
            "Columns",
            "Stack",
            "ButtonRow",
            "ImagePlaceholder",
            "Panel",
        ] {
            assert!(
                all.iter().any(|name| name == expected),
                "missing {expected}"
            );
        }
    }

    fn doc(path: &str, title: &str) -> DocumentMeta {
        DocumentMeta {
            id: flynt_core::models::DocumentId::new(),
            path: path.into(),
            title: title.to_string(),
            tags: Vec::new(),
            metadata: Default::default(),
            entity_kind: None,
            updated_at: chrono::Utc::now(),
        }
    }

    #[test]
    fn detects_design_board_wrappers_in_boards_directory() {
        assert_eq!(
            design_artifact_kind(Path::new("boards/Hero.md")),
            Some(DesignArtifactKind::Board)
        );
    }

    #[test]
    fn does_not_treat_canvas_wrappers_as_design_boards() {
        assert_eq!(design_artifact_kind(Path::new("canvases/Hero.md")), None);
    }

    #[test]
    fn detects_excalidraw_drawing_wrappers() {
        assert_eq!(
            design_artifact_kind(Path::new("drawings/System Map.md")),
            Some(DesignArtifactKind::Drawing)
        );
    }

    #[test]
    fn rejects_non_markdown_files_in_drawing_directory() {
        assert_eq!(
            design_artifact_kind(Path::new("drawings/System Map.excalidraw")),
            None
        );
        assert_eq!(
            design_artifact_kind(Path::new("drawings/System Map.svg")),
            None
        );
    }

    #[test]
    fn detects_flow_files_anywhere() {
        assert_eq!(
            design_artifact_kind(Path::new("flows/Auth.flow")),
            Some(DesignArtifactKind::Flow)
        );
        assert_eq!(
            design_artifact_kind(Path::new("docs/Auth.flow")),
            Some(DesignArtifactKind::Flow)
        );
    }

    #[test]
    fn rejects_regular_markdown_notes() {
        assert_eq!(
            design_artifact_kind(Path::new("docs/Architecture.md")),
            None
        );
        assert_eq!(
            design_artifact_kind(Path::new("boards/Architecture.txt")),
            None
        );
    }

    #[test]
    fn lists_only_design_artifacts_and_sorts_by_path() {
        let docs = vec![
            doc("docs/Architecture.md", "Architecture"),
            doc("drawings/System.md", "System"),
            doc("boards/Hero.md", "Hero"),
            doc("flows/Auth.flow", "Auth"),
            doc("drawings/System.excalidraw", "System backing"),
        ];

        let artifacts = design_artifacts(&docs);
        let paths: Vec<_> = artifacts
            .iter()
            .map(|artifact| artifact.doc.path.to_string_lossy().to_string())
            .collect();
        let kinds: Vec<_> = artifacts.iter().map(|artifact| artifact.kind).collect();

        assert_eq!(
            paths,
            vec!["boards/Hero.md", "drawings/System.md", "flows/Auth.flow"]
        );
        assert_eq!(
            kinds,
            vec![
                DesignArtifactKind::Board,
                DesignArtifactKind::Drawing,
                DesignArtifactKind::Flow,
            ]
        );
    }

    #[test]
    fn artifact_title_prefers_file_stem_over_frontmatter_title() {
        let meta = doc("boards/Hero Board.md", "Frontmatter Title");
        assert_eq!(artifact_title(&meta), "Hero Board");
    }
}
