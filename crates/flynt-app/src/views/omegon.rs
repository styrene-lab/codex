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
    let journal_entries = load_journal_timeline(&journal);
    let journal_doc = ctx
        .project()
        .store
        .get_document_by_path(std::path::Path::new(".omegon/agent-journal.md"))
        .ok()
        .flatten();
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
                div { class: "omegon-surface-card omegon-journal-card",
                    div { class: "omegon-surface-card-head",
                        div {
                            span { class: "omegon-surface-kicker", "Journal" }
                            h2 { "Agent Journal" }
                        }
                        span { class: "omegon-surface-meta", "{journal_size} bytes" }
                    }
                    p { if journal_available { "Chronological record of agent sessions and outcomes." } else { "No project agent journal found yet." } }
                    if journal_available {
                        div { class: "omegon-journal-timeline",
                            for entry in journal_entries.iter() {
                                article { class: "omegon-journal-entry",
                                    div { class: "omegon-journal-entry-head",
                                        span { class: "omegon-journal-entry-title", "{entry.title}" }
                                        if let Some(timestamp) = entry.timestamp.as_ref() {
                                            span { class: "omegon-journal-entry-time", "{timestamp}" }
                                        }
                                    }
                                    if let Some(objective) = entry.objective.as_ref() {
                                        div { class: "omegon-journal-field",
                                            span { "Objective" }
                                            p { "{objective}" }
                                        }
                                    }
                                    if let Some(outcome) = entry.outcome.as_ref() {
                                        div { class: "omegon-journal-field",
                                            span { "Outcome" }
                                            p { "{outcome}" }
                                        }
                                    }
                                    if !entry.notes.is_empty() {
                                        div { class: "omegon-journal-notes",
                                            for note in entry.notes.iter() {
                                                div { class: "omegon-journal-line", "{note}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
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

/// Curated hidden-artifact policy for `.omegon`:
/// - Agent journal is operator-facing project history and may be indexed/opened.
/// - Runtime JSON, plugin manifests/personas, workflows, and deployment manifests stay out of
///   normal Notes/Files and are projected here as structured operational views.
/// - Reveal actions remain debug affordances, not the primary UX.
#[derive(Clone, Debug, PartialEq, Eq)]
struct JournalTimelineEntry {
    timestamp: Option<String>,
    title: String,
    objective: Option<String>,
    outcome: Option<String>,
    notes: Vec<String>,
}

fn load_journal_timeline(path: &std::path::Path) -> Vec<JournalTimelineEntry> {
    let Ok(content) = std::fs::read_to_string(path) else {
        return vec![empty_journal_entry()];
    };
    let entries = parse_journal_entries(&content);
    if entries.is_empty() {
        vec![empty_journal_entry()]
    } else {
        entries.into_iter().take(4).collect()
    }
}

fn parse_journal_entries(content: &str) -> Vec<JournalTimelineEntry> {
    let body_lines: Vec<&str> = content
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !(trimmed == "---" || trimmed.starts_with("title:") || trimmed.starts_with("tags:"))
        })
        .collect();

    let mut entries = Vec::new();
    let mut current: Option<JournalTimelineEntry> = None;
    let mut fallback_notes = Vec::new();

    for line in body_lines {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed == "# Agent Journal" {
            continue;
        }
        if let Some(heading) = trimmed.strip_prefix("## ") {
            if let Some(entry) = current.take() {
                entries.push(entry);
            }
            current = Some(entry_from_heading(heading));
            continue;
        }

        let note = trimmed.strip_prefix("- ").unwrap_or(trimmed).trim();
        if note.is_empty() {
            continue;
        }
        if let Some(entry) = current.as_mut() {
            apply_journal_line(entry, note);
        } else {
            fallback_notes.push(note.to_string());
        }
    }
    if let Some(entry) = current.take() {
        entries.push(entry);
    }

    if entries.is_empty() && !fallback_notes.is_empty() {
        let mut entry = JournalTimelineEntry {
            timestamp: None,
            title: "Current journal state".to_string(),
            objective: None,
            outcome: None,
            notes: Vec::new(),
        };
        for note in fallback_notes {
            apply_journal_line(&mut entry, &note);
        }
        entries.push(entry);
    }

    entries.into_iter().rev().collect()
}

fn entry_from_heading(heading: &str) -> JournalTimelineEntry {
    let (timestamp, title) = heading
        .split_once('—')
        .or_else(|| heading.split_once(" - "))
        .map(|(left, right)| (Some(left.trim().to_string()), right.trim().to_string()))
        .unwrap_or((None, heading.trim().to_string()));
    JournalTimelineEntry {
        timestamp,
        title,
        objective: None,
        outcome: None,
        notes: Vec::new(),
    }
}

fn apply_journal_line(entry: &mut JournalTimelineEntry, line: &str) {
    if let Some(value) = strip_field(line, "Objective").or_else(|| strip_field(line, "Current objective")) {
        entry.objective = Some(value.to_string());
    } else if let Some(value) = strip_field(line, "Outcome") {
        entry.outcome = Some(value.to_string());
    } else {
        entry.notes.push(line.to_string());
    }
}

fn strip_field<'a>(line: &'a str, label: &str) -> Option<&'a str> {
    line.strip_prefix(&format!("**{label}:**"))
        .or_else(|| line.strip_prefix(&format!("{label}:")))
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn empty_journal_entry() -> JournalTimelineEntry {
    JournalTimelineEntry {
        timestamp: None,
        title: "No journal entries yet".to_string(),
        objective: None,
        outcome: None,
        notes: vec!["Omegon has not recorded project-local agent history yet.".to_string()],
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
