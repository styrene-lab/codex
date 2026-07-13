use crate::apple_notes::{self, AppleNotesCatalog};
use dioxus::prelude::*;

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

#[component]
pub fn AppleNotesImportSection() -> Element {
    let mut loading = use_signal(|| false);
    let mut catalog = use_signal(|| Option::<AppleNotesCatalog>::None);
    let mut error = use_signal(|| Option::<String>::None);

    let discover = move |_| {
        if *loading.read() {
            return;
        }
        *loading.write() = true;
        *error.write() = None;
        spawn(async move {
            match apple_notes::discover().await {
                Ok(value) => *catalog.write() = Some(value),
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
                if let Some((folders, notes)) = counts {
                    div { class: "settings-card",
                        h4 { "Apple Notes is ready" }
                        p { "Found {notes} notes across {folders} folders in {catalog.read().as_ref().map_or(0, |value| value.accounts.len())} accounts." }
                        p { class: "settings-hint",
                            "The catalog contains metadata only. Selection and import preview are the next step; no note has been copied yet."
                        }
                    }
                }
            }
        }
    }
}
