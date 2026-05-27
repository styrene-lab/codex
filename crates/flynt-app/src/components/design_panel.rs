use dioxus::prelude::*;
use std::collections::BTreeMap;

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

    rsx! {
        div { class: "design-panel",
            section { class: "design-panel-section",
                div { class: "design-panel-heading", "Design" }
                p { class: "design-panel-hint",
                    "Visual surfaces stay plain-text .board JSON; exports are generated outputs."
                }
            }

            section { class: "design-panel-section",
                div { class: "design-panel-subheading", "Files" }
                div { class: "design-panel-placeholder", "Boards, drawings, and flows will appear here next." }
            }

            section { class: "design-panel-section",
                div { class: "design-panel-subheading", "Surface kinds" }
                div { class: "design-profile-list",
                    for profile in profiles.iter() {
                        div { key: "{profile.kind.as_str()}", class: "design-profile-card",
                            div { class: "design-profile-title", "{profile.label}" }
                            div { class: "design-profile-description", "{profile.description}" }
                            div { class: "design-profile-meta",
                                span { "source: {profile.source_format}" }
                                span { "exports: {profile.export_targets.join(\", \")}" }
                            }
                        }
                    }
                }
            }

            section { class: "design-panel-section",
                div { class: "design-panel-subheading", "Components" }
                for group in groups.iter() {
                    div { key: "{group.category}", class: "design-component-group",
                        div { class: "design-component-category", "{group.category}" }
                        for component in group.components.iter() {
                            div { key: "{component.name}", class: "design-component-card",
                                div { class: "design-component-title", "{component.name}" }
                                div { class: "design-component-description", "{component.description}" }
                                div { class: "design-component-variants", "variants: {component.variants.join(\", \")}" }
                            }
                        }
                    }
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
