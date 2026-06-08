use crate::{
    bootstrap::AppContext,
    state::{Route, SettingsOpen, SettingsPage, TabState},
};
use dioxus::prelude::*;
use flynt_core::store::ProjectStore;

#[component]
pub fn OmegonProjectView() -> Element {
    let ctx = use_context::<AppContext>();
    let mut tab_state = use_context::<Signal<TabState>>();
    let mut active_route = use_context::<Signal<Route>>();
    let mut settings_page = use_context::<Signal<SettingsPage>>();
    let mut settings_open = use_context::<Signal<SettingsOpen>>();

    let root = ctx.project_root().join(".omegon");
    let journal = root.join("agent-journal.md");
    let plugins = root.join("plugins");
    let deployment = ctx.omegon().deployment_path;
    let journal_meta = std::fs::metadata(&journal).ok();
    let deployment_meta = std::fs::metadata(&deployment).ok();
    let plugin_count = std::fs::read_dir(&plugins)
        .map(|entries| entries.filter_map(Result::ok).count())
        .unwrap_or(0);
    let journal_size = journal_meta.as_ref().map(|meta| meta.len()).unwrap_or(0);
    let deployment_size = deployment_meta.as_ref().map(|meta| meta.len()).unwrap_or(0);
    let journal_doc = ctx.project().store.list_documents().ok().and_then(|docs| {
        docs.into_iter()
            .find(|doc| doc.path == std::path::Path::new(".omegon/agent-journal.md"))
    });
    let journal_available = journal_meta.is_some();

    rsx! {
        div { class: "omegon-surface",
            div { class: "omegon-surface-header",
                div {
                    h1 { "Omegon" }
                    p { "Project-local agent state, memory artifacts, plugins, and operational journal." }
                }
                div { class: "omegon-surface-actions",
                    button {
                        class: "btn btn-sm btn-ghost",
                        onclick: move |_| reveal_path(&root),
                        "Reveal .omegon"
                    }
                    button {
                        class: "btn btn-sm btn-primary",
                        onclick: move |_| {
                            *settings_page.write() = SettingsPage::OmegonRuntime;
                            *settings_open.write() = SettingsOpen(true);
                        },
                        "Runtime settings"
                    }
                }
            }
            div { class: "omegon-surface-grid",
                div { class: "omegon-surface-card",
                    span { class: "omegon-surface-kicker", "Journal" }
                    h2 { "Agent Journal" }
                    p { if journal_available { "Chronological record of agent sessions and outcomes." } else { "No project agent journal found yet." } }
                    span { class: "omegon-surface-meta", "{journal_size} bytes" }
                    div { class: "omegon-surface-card-actions",
                        button {
                            class: "btn btn-xs btn-primary",
                            disabled: journal_doc.is_none(),
                            onclick: move |_| {
                                if let Some(doc) = journal_doc.clone() {
                                    tab_state.write().open(doc.id.clone(), doc.title.clone());
                                    *active_route.write() = Route::Notes;
                                }
                            },
                            "Open journal"
                        }
                        button { class: "btn btn-xs btn-ghost", disabled: !journal_available, onclick: move |_| reveal_path(&journal), "Reveal" }
                    }
                }
                div { class: "omegon-surface-card",
                    span { class: "omegon-surface-kicker", "Deployment" }
                    h2 { "ACP Manifest" }
                    p { if deployment_meta.is_some() { "Project-scoped ACP profile, memory, and extension contract." } else { "Deployment manifest has not been created yet." } }
                    span { class: "omegon-surface-meta", "{deployment_size} bytes" }
                    div { class: "omegon-surface-card-actions",
                        button { class: "btn btn-xs btn-ghost", disabled: deployment_meta.is_none(), onclick: move |_| reveal_path(&deployment), "Reveal manifest" }
                    }
                }
                div { class: "omegon-surface-card",
                    span { class: "omegon-surface-kicker", "Runtime" }
                    h2 { "Plugins" }
                    p { "Project-scoped Omegon plugin checkout/cache area." }
                    span { class: "omegon-surface-meta", "{plugin_count} item(s)" }
                    div { class: "omegon-surface-card-actions",
                        button { class: "btn btn-xs btn-ghost", onclick: move |_| reveal_path(&plugins), "Reveal plugins" }
                    }
                }
            }
        }
    }
}

fn reveal_path(path: &std::path::Path) {
    #[cfg(target_os = "macos")]
    {
        let mut command = std::process::Command::new("open");
        if path.is_dir() {
            command.arg(path);
        } else {
            command.arg("-R").arg(path);
        }
        let _ = command.spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let target = if path.is_dir() {
            path
        } else {
            path.parent().unwrap_or(path)
        };
        let _ = std::process::Command::new("xdg-open").arg(target).spawn();
    }
}
