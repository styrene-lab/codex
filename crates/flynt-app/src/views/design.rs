use dioxus::prelude::*;

#[component]
pub fn DesignView() -> Element {
    let profile_count = flynt_core::design_surfaces::list_surface_profiles().len();
    let component_count = flynt_core::design_components::list_components().len();

    rsx! {
        div { class: "design-view",
            div { class: "design-view-hero",
                div { class: "design-view-kicker", "Design mode" }
                h1 { "Visual surfaces" }
                p {
                    "Use the left Design panel to create and open design boards, drawings, and flows, inspect surface kinds, and browse available components."
                }
                div { class: "design-view-stats",
                    div { class: "design-view-stat",
                        strong { "{profile_count}" }
                        span { "surface kinds" }
                    }
                    div { class: "design-view-stat",
                        strong { "{component_count}" }
                        span { "components" }
                    }
                    div { class: "design-view-stat",
                        strong { ".board" }
                        span { "board backing file" }
                    }
                }
            }
        }
    }
}
