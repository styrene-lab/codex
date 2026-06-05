use dioxus::prelude::*;

#[component]
pub fn DesignView() -> Element {
    let profiles = flynt_core::design_surfaces::list_surface_profiles();
    let components = flynt_core::design_components::list_components();
    let component_count = components.len();
    let profile_count = profiles.len();
    let featured_components: Vec<_> = components.iter().take(6).collect();
    let surface_cards: Vec<_> = profiles
        .iter()
        .take(6)
        .map(|profile| {
            (
                profile.label,
                profile.description,
                profile.export_targets.join(" / "),
            )
        })
        .collect();

    rsx! {
        div { class: "design-view design-view-dashboard",
            section { class: "design-dashboard-hero",
                div { class: "design-view-kicker", "Design workspace" }
                h1 { "Build visual surfaces" }
                p {
                    "Create design boards, drawings, and flows from the left panel, then use components as structured building blocks for fast mockups."
                }
                div { class: "design-action-grid",
                    div { class: "design-action-card primary",
                        strong { "Design boards" }
                        span { "Structured UI mockups and component layouts. Create one from Files → New Design Board." }
                    }
                    div { class: "design-action-card",
                        strong { "Drawings" }
                        span { "Freeform sketches and spatial diagrams. Create one from Files → New Drawing." }
                    }
                    div { class: "design-action-card",
                        strong { "Flows" }
                        span { "Node graphs for processes and systems. Create one from Files → New Flow." }
                    }
                }
            }

            section { class: "design-dashboard-section",
                div { class: "design-section-head",
                    h2 { "Surface types" }
                    span { "{profile_count} available" }
                }
                div { class: "design-surface-grid",
                    for (label, description, exports) in surface_cards.iter() {
                        div { class: "design-surface-card",
                            strong { "{label}" }
                            p { "{description}" }
                            span { "Exports: {exports}" }
                        }
                    }
                }
            }

            section { class: "design-dashboard-section",
                div { class: "design-section-head",
                    h2 { "Component catalog" }
                    span { "{component_count} components" }
                }
                div { class: "design-component-preview-grid",
                    for component in featured_components {
                        div { class: "design-component-preview-card",
                            div { class: "design-component-preview-top",
                                strong { "{component.name}" }
                                span { "{component.category}" }
                            }
                            p { "{component.description}" }
                            div { class: "design-component-variants", "{component.variants.len()} variant(s)" }
                        }
                    }
                }
            }
        }
    }
}
