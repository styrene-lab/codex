use dioxus::prelude::*;
use std::collections::BTreeMap;

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
                variants: component.variants.iter().map(|variant| variant.to_string()).collect(),
            });
    }
    by_category
        .into_iter()
        .map(|(category, mut components)| {
            components.sort_by(|a, b| a.name.cmp(&b.name));
            ComponentGroup { category, components }
        })
        .collect()
}

#[component]
pub fn DesignPanel() -> Element {
    let profiles = flynt_core::design_surfaces::list_surface_profiles();
    let groups = grouped_component_summaries();
    let mut tab = use_signal(|| DesignPanelTab::Files);

    rsx! {
        div { class: "design-panel",
            section { class: "design-panel-section design-panel-intro",
                div { class: "design-panel-heading", "Design" }
                p { class: "design-panel-hint",
                    "Boards, drawings, flows, templates, and components for visual work."
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
                    DesignFilesTab {}
                },
                DesignPanelTab::Kinds => rsx! {
                    section { class: "design-panel-section",
                        div { class: "design-panel-subheading", "Surface kinds" }
                        div { class: "design-kind-list",
                            for profile in profiles.iter() {
                                button { key: "{profile.kind.as_str()}", class: "design-kind-row",
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
fn DesignFilesTab() -> Element {
    rsx! {
        section { class: "design-panel-section",
            div { class: "design-panel-subheading", "Files" }
            div { class: "design-file-actions",
                button { class: "design-action-button primary", "New Design Board" }
                button { class: "design-action-button", "New Drawing" }
                button { class: "design-action-button", "New Flow" }
            }
            div { class: "design-empty-state",
                div { class: "design-empty-title", "No design artifact browser yet" }
                p { "This shell is wired. Next pass will list .board wrappers, Excalidraw drawings, and .flow diagrams here." }
            }
            div { class: "design-panel-subheading", "Quick orientation" }
            div { class: "design-quick-list",
                div { class: "design-quick-row",
                    span { "Canonical source" }
                    strong { "board_json" }
                }
                div { class: "design-quick-row",
                    span { "Default workflow" }
                    strong { "create → revise → export" }
                }
                div { class: "design-quick-row",
                    span { "Editable now" }
                    strong { "components + raw HTML" }
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
            assert!(all.iter().any(|name| name == expected), "missing {expected}");
        }
    }
}
