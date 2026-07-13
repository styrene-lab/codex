use crate::{
    apple_notes::{
        self, AppleNoteSummary, AppleNotesCatalog, AppleNotesImportReport, PreparedAppleNote,
    },
    bootstrap::AppContext,
};
use dioxus::prelude::*;
use std::collections::HashSet;

fn catalog_counts(catalog: &AppleNotesCatalog) -> (usize, usize) {
    fn folder_count(folder: &apple_notes::AppleNotesFolder) -> (usize, usize) {
        folder
            .folders
            .iter()
            .fold((1, folder.notes.len()), |(folders, notes), child| {
                let (child_folders, child_notes) = folder_count(child);
                (folders + child_folders, notes + child_notes)
            })
    }
    catalog.accounts.iter().fold((0, 0), |counts, account| {
        account
            .folders
            .iter()
            .fold(counts, |(folders, notes), folder| {
                let (child_folders, child_notes) = folder_count(folder);
                (folders + child_folders, notes + child_notes)
            })
    })
}

fn flatten_catalog(catalog: &AppleNotesCatalog) -> Vec<AppleNoteSummary> {
    fn collect(folder: &apple_notes::AppleNotesFolder, output: &mut Vec<AppleNoteSummary>) {
        output.extend(folder.notes.iter().cloned());
        for child in &folder.folders {
            collect(child, output);
        }
    }
    let mut notes = Vec::new();
    for account in &catalog.accounts {
        for folder in &account.folders {
            collect(folder, &mut notes);
        }
    }
    notes.sort_by(|left, right| right.modified_at.cmp(&left.modified_at));
    notes
}

#[component]
pub fn AppleNotesImportSection() -> Element {
    let mut loading = use_signal(|| false);
    let mut catalog = use_signal(|| Option::<AppleNotesCatalog>::None);
    let mut error = use_signal(|| Option::<String>::None);
    let mut selected = use_signal(HashSet::<String>::new);
    let mut preview = use_signal(Vec::<PreparedAppleNote>::new);
    let mut report = use_signal(|| Option::<AppleNotesImportReport>::None);
    let project = consume_context::<AppContext>().project();

    let discover = move |_| {
        if *loading.read() {
            return;
        }
        *loading.write() = true;
        *error.write() = None;
        spawn(async move {
            match apple_notes::discover().await {
                Ok(value) => {
                    *catalog.write() = Some(value);
                    selected.write().clear();
                    preview.write().clear();
                    *report.write() = None;
                }
                Err(apple_notes::AppleNotesError::PermissionDenied) => {
                    *error.write() = Some(
                        "Permission denied. Open System Settings → Privacy & Security → Automation, allow Flynt to control Notes, then try again."
                            .into(),
                    );
                }
                Err(err) => *error.write() = Some(err.to_string()),
            }
            *loading.write() = false;
        });
    };

    let counts = catalog.read().as_ref().map(catalog_counts);
    let notes = catalog
        .read()
        .as_ref()
        .map(flatten_catalog)
        .unwrap_or_default();
    let selected_count = selected.read().len();
    rsx! {
        div { class: "settings-page",
            h2 { "Import" }
            p { class: "settings-description",
                "Copy selected material into this Flynt project. Source applications remain unchanged."
            }
            section { class: "settings-section",
                h3 { "Apple Notes" }
                p {
                    "Flynt first reads note titles, folders, dates, and attachment counts. Note bodies are requested only after you select what to import."
                }
                if !apple_notes::is_available() {
                    p { class: "settings-hint", "Apple Notes import is available in the macOS app." }
                } else {
                    button {
                        class: "settings-button primary",
                        disabled: *loading.read(),
                        onclick: discover,
                        if *loading.read() { "Reading Apple Notes…" } else { "Browse Apple Notes" }
                    }
                }
                if let Some(message) = error.read().as_ref() {
                    p { class: "settings-message err", "{message}" }
                }
                if let Some((folders, note_count)) = counts {
                    div { class: "settings-card",
                        h4 { "Apple Notes is ready" }
                        p { "Found {note_count} notes across {folders} folders in {catalog.read().as_ref().map_or(0, |value| value.accounts.len())} accounts." }
                        p { class: "settings-hint", "Select up to 100 notes. Bodies are read only when you create the preview." }
                    }
                    div { class: "apple-notes-catalog",
                        for note in notes.iter() {
                            {
                                let note_id = note.id.clone();
                                let checked = selected.read().contains(&note.id);
                                rsx! {
                                    label { class: "apple-notes-row",
                                        input {
                                            r#type: "checkbox",
                                            checked,
                                            disabled: !checked && selected_count >= 100,
                                            onchange: move |event| {
                                                if event.checked() {
                                                    selected.write().insert(note_id.clone());
                                                } else {
                                                    selected.write().remove(&note_id);
                                                }
                                                preview.write().clear();
                                                *report.write() = None;
                                            }
                                        }
                                        span { class: "apple-notes-row-copy",
                                            strong { "{note.name}" }
                                            small { "{note.folder_path} · modified {note.modified_at}" }
                                        }
                                        if note.password_protected { span { class: "settings-badge", "Locked" } }
                                        if note.shared { span { class: "settings-badge", "Shared" } }
                                        if note.attachment_count > 0 { span { class: "settings-badge", "{note.attachment_count} attachments" } }
                                    }
                                }
                            }
                        }
                    }
                    div { class: "settings-actions",
                        button {
                            class: "settings-button primary",
                            disabled: selected_count == 0 || *loading.read(),
                            onclick: move |_| {
                                let ids = selected.read().iter().cloned().collect::<Vec<_>>();
                                *loading.write() = true;
                                *error.write() = None;
                                spawn(async move {
                                    match apple_notes::export_selected(&ids).await {
                                        Ok(exported) => {
                                            *preview.write() = exported.notes.into_iter().map(apple_notes::prepare_note).collect();
                                        }
                                        Err(err) => *error.write() = Some(err.to_string()),
                                    }
                                    *loading.write() = false;
                                });
                            },
                            "Preview {selected_count} selected"
                        }
                    }
                }
                if !preview.read().is_empty() {
                    section { class: "settings-section",
                        h3 { "Import preview" }
                        p { "Review the staged Markdown. Import creates independent Flynt copies under Apple Notes Import/." }
                        for note in preview.read().iter() {
                            article { class: "settings-card apple-notes-preview",
                                h4 { "{note.title}" }
                                p { class: "settings-hint", "{note.folder_path}" }
                                if note.markdown.is_empty() {
                                    p { class: "settings-message warn", "This locked note will be skipped." }
                                } else {
                                    pre { "{note.markdown}" }
                                }
                                for warning in &note.warnings { p { class: "settings-message warn", "{warning}" } }
                            }
                        }
                        button {
                            class: "settings-button primary",
                            onclick: move |_| {
                                match apple_notes::import_prepared_notes(&project, preview.read().clone()) {
                                    Ok(value) => *report.write() = Some(value),
                                    Err(err) => *error.write() = Some(err.to_string()),
                                }
                            },
                            "Import into Flynt"
                        }
                    }
                }
                if let Some(value) = report.read().as_ref() {
                    p { class: "settings-message ok",
                        "Imported {value.imported.len()} notes; skipped {value.skipped_locked} locked notes. Apple Notes was not changed."
                    }
                }
            }
        }
    }
}
