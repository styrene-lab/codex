//! macOS Apple Notes metadata discovery through the supported Notes scripting API.
//!
//! This module deliberately exposes summaries only. Importing bodies and attachments
//! is a separate step so opening the picker does not traverse private content.

use flynt_core::{
    models::{Frontmatter, MetadataValue},
    store::ProjectStore,
};
use flynt_store::project::Project;
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
    process::Stdio,
    time::Duration,
};
use thiserror::Error;
use uuid::Uuid;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
const EXPORT_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_SELECTED_NOTES: usize = 250;

/// Fixed JXA program. Values from Notes are serialized by JSON.stringify; no
/// operator-controlled value is interpolated into this source.
const DISCOVER_SCRIPT: &str = r#"
ObjC.import('Foundation');
const Notes = Application('Notes');
Notes.includeStandardAdditions = false;

function text(value) {
  return value === null || value === undefined ? '' : String(value);
}

function summarizeFolder(folder, accountId, parentPath) {
  const name = text(folder.name());
  const path = parentPath ? parentPath + '/' + name : name;
  const notes = folder.notes().map(note => ({
    id: text(note.id()),
    name: text(note.name()),
    account_id: accountId,
    folder_id: text(folder.id()),
    folder_path: path,
    created_at: note.creationDate().toISOString(),
    modified_at: note.modificationDate().toISOString(),
    password_protected: Boolean(note.passwordProtected()),
    shared: Boolean(note.shared()),
    attachment_count: note.attachments().length
  }));
  return {
    id: text(folder.id()),
    name: name,
    path: path,
    notes: notes,
    folders: folder.folders().map(child => summarizeFolder(child, accountId, path))
  };
}

const accounts = Notes.accounts().map(account => {
  const accountId = text(account.id());
  return {
    id: accountId,
    name: text(account.name()),
    folders: account.folders().map(folder => summarizeFolder(folder, accountId, ''))
  };
});
JSON.stringify({schema_version: 1, accounts: accounts});
"#;

/// Export is selected by stable IDs passed as one JSON process argument. The
/// fixed program never embeds operator-controlled note names or identifiers.
const EXPORT_SCRIPT: &str = r#"
ObjC.import('Foundation');
const Notes = Application('Notes');
Notes.includeStandardAdditions = false;

function text(value) {
  return value === null || value === undefined ? '' : String(value);
}

function iso(value) {
  return value && typeof value.toISOString === 'function' ? value.toISOString() : '';
}

function collectFolder(folder, accountId, parentPath, selected, records) {
  const name = text(folder.name());
  const path = parentPath ? parentPath + '/' + name : name;
  folder.notes().forEach(note => {
    const id = text(note.id());
    if (!selected.has(id)) return;
    records.push({
      id: id,
      name: text(note.name()),
      account_id: accountId,
      folder_id: text(folder.id()),
      folder_path: path,
      created_at: iso(note.creationDate()),
      modified_at: iso(note.modificationDate()),
      password_protected: Boolean(note.passwordProtected()),
      shared: Boolean(note.shared()),
      html: Boolean(note.passwordProtected()) ? '' : text(note.body()),
      plaintext: Boolean(note.passwordProtected()) ? '' : text(note.plaintext()),
      attachments: note.attachments().map(attachment => ({
        id: text(attachment.id()),
        name: text(attachment.name()),
        content_id: text(attachment.contentIdentifier()),
        url: text(attachment.URL()),
        created_at: iso(attachment.creationDate()),
        modified_at: iso(attachment.modificationDate())
      }))
    });
  });
  folder.folders().forEach(child => collectFolder(child, accountId, path, selected, records));
}

let ids;
try {
  ids = JSON.parse($.NSProcessInfo.processInfo.arguments.objectAtIndex(4).js);
} catch (error) {
  throw new Error('invalid selected note id payload');
}
if (!Array.isArray(ids) || ids.some(id => typeof id !== 'string')) {
  throw new Error('selected note ids must be a string array');
}
const selected = new Set(ids);
const records = [];
Notes.accounts().forEach(account => {
  const accountId = text(account.id());
  account.folders().forEach(folder => collectFolder(folder, accountId, '', selected, records));
});
JSON.stringify({schema_version: 1, requested: ids.length, notes: records});
"#;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct AppleNotesCatalog {
    pub schema_version: u32,
    pub accounts: Vec<AppleNotesAccount>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct AppleNotesExport {
    pub schema_version: u32,
    pub requested: usize,
    pub notes: Vec<AppleNoteExportRecord>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct AppleNoteExportRecord {
    pub id: String,
    pub name: String,
    pub account_id: String,
    pub folder_id: String,
    pub folder_path: String,
    pub created_at: String,
    pub modified_at: String,
    pub password_protected: bool,
    pub shared: bool,
    pub html: String,
    pub plaintext: String,
    #[serde(default)]
    pub attachments: Vec<AppleNoteAttachment>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct AppleNoteAttachment {
    pub id: String,
    pub name: String,
    pub content_id: String,
    pub url: String,
    pub created_at: String,
    pub modified_at: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparedAppleNote {
    pub source_id: String,
    pub account_id: String,
    pub title: String,
    pub folder_path: String,
    pub markdown: String,
    pub created_at: String,
    pub modified_at: String,
    pub shared: bool,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ImportedAppleNote {
    pub source_id: String,
    pub path: PathBuf,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AppleNotesImportReport {
    pub imported: Vec<ImportedAppleNote>,
    pub skipped_locked: usize,
    pub skipped_existing: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct AppleNotesAccount {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub folders: Vec<AppleNotesFolder>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct AppleNotesFolder {
    pub id: String,
    pub name: String,
    pub path: String,
    #[serde(default)]
    pub notes: Vec<AppleNoteSummary>,
    #[serde(default)]
    pub folders: Vec<AppleNotesFolder>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct AppleNoteSummary {
    pub id: String,
    pub name: String,
    pub account_id: String,
    pub folder_id: String,
    pub folder_path: String,
    pub created_at: String,
    pub modified_at: String,
    pub password_protected: bool,
    pub shared: bool,
    pub attachment_count: usize,
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum AppleNotesError {
    #[error("Apple Notes import is available only on macOS")]
    UnsupportedPlatform,
    #[error("Apple Notes is unavailable on this Mac")]
    NotesUnavailable,
    #[error("Flynt does not have permission to read Apple Notes")]
    PermissionDenied,
    #[error("Apple Notes did not respond within {seconds} seconds")]
    Timeout { seconds: u64 },
    #[error("Select at most {max} Apple Notes at a time")]
    SelectionTooLarge { max: usize },
    #[error("Apple Notes did not return all selected notes ({returned}/{requested})")]
    IncompleteExport { requested: usize, returned: usize },
    #[error("Apple Notes discovery failed: {message}")]
    Process { message: String },
    #[error("Apple Notes returned an invalid discovery response")]
    MalformedResponse,
    #[error("Apple Notes returned unsupported schema version {0}")]
    UnsupportedSchema(u32),
}

pub fn is_available() -> bool {
    cfg!(target_os = "macos")
        && Path::new("/usr/bin/osascript").is_file()
        && Path::new("/System/Applications/Notes.app").exists()
}

pub async fn discover() -> Result<AppleNotesCatalog, AppleNotesError> {
    discover_with_timeout(DEFAULT_TIMEOUT).await
}

async fn discover_with_timeout(timeout: Duration) -> Result<AppleNotesCatalog, AppleNotesError> {
    if !cfg!(target_os = "macos") {
        return Err(AppleNotesError::UnsupportedPlatform);
    }
    if !is_available() {
        return Err(AppleNotesError::NotesUnavailable);
    }

    let mut command = tokio::process::Command::new("/usr/bin/osascript");
    command
        .args(["-l", "JavaScript", "-e", DISCOVER_SCRIPT])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);

    let output = tokio::time::timeout(timeout, command.output())
        .await
        .map_err(|_| AppleNotesError::Timeout {
            seconds: timeout.as_secs(),
        })?
        .map_err(|error| AppleNotesError::Process {
            message: sanitize_process_error(&error.to_string()),
        })?;

    if !output.status.success() {
        return Err(classify_process_failure(&String::from_utf8_lossy(
            &output.stderr,
        )));
    }

    parse_catalog(&output.stdout)
}

pub async fn export_selected(note_ids: &[String]) -> Result<AppleNotesExport, AppleNotesError> {
    if note_ids.len() > MAX_SELECTED_NOTES {
        return Err(AppleNotesError::SelectionTooLarge {
            max: MAX_SELECTED_NOTES,
        });
    }
    if !is_available() {
        return Err(if cfg!(target_os = "macos") {
            AppleNotesError::NotesUnavailable
        } else {
            AppleNotesError::UnsupportedPlatform
        });
    }

    let payload =
        serde_json::to_string(note_ids).map_err(|_| AppleNotesError::MalformedResponse)?;
    let output = tokio::time::timeout(
        EXPORT_TIMEOUT,
        tokio::process::Command::new("/usr/bin/osascript")
            .args(["-l", "JavaScript", "-e", EXPORT_SCRIPT, "--", &payload])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .kill_on_drop(true)
            .output(),
    )
    .await
    .map_err(|_| AppleNotesError::Timeout {
        seconds: EXPORT_TIMEOUT.as_secs(),
    })?
    .map_err(|error| AppleNotesError::Process {
        message: sanitize_process_error(&error.to_string()),
    })?;
    if !output.status.success() {
        return Err(classify_process_failure(&String::from_utf8_lossy(
            &output.stderr,
        )));
    }
    let exported: AppleNotesExport =
        serde_json::from_slice(&output.stdout).map_err(|_| AppleNotesError::MalformedResponse)?;
    if exported.schema_version != 1 {
        return Err(AppleNotesError::UnsupportedSchema(exported.schema_version));
    }
    if exported.requested != note_ids.len() || exported.notes.len() != note_ids.len() {
        return Err(AppleNotesError::IncompleteExport {
            requested: note_ids.len(),
            returned: exported.notes.len(),
        });
    }
    Ok(exported)
}

pub fn prepare_note(record: AppleNoteExportRecord) -> PreparedAppleNote {
    let mut warnings = Vec::new();
    if record.shared {
        warnings.push("shared note imported as an independent snapshot".to_string());
    }
    if !record.attachments.is_empty() {
        warnings.push(format!(
            "{} attachment(s) require a later binary export pass",
            record.attachments.len()
        ));
    }
    let markdown = if record.password_protected {
        warnings.push("password-protected note skipped".to_string());
        String::new()
    } else {
        html2md_rs::to_md::safe_from_html_to_md(record.html.clone())
            .unwrap_or_else(|_| {
                warnings.push("rich text conversion failed; used plain text".to_string());
                record.plaintext.clone()
            })
            .trim()
            .to_string()
    };
    PreparedAppleNote {
        source_id: record.id,
        account_id: record.account_id,
        title: record.name,
        folder_path: record.folder_path,
        markdown,
        created_at: record.created_at,
        modified_at: record.modified_at,
        shared: record.shared,
        warnings,
    }
}

pub fn import_prepared_notes(
    project: &Project,
    notes: Vec<PreparedAppleNote>,
) -> Result<AppleNotesImportReport, AppleNotesError> {
    let mut report = AppleNotesImportReport::default();
    let existing_source_ids: HashSet<String> = project
        .store
        .list_documents()
        .map_err(|error| AppleNotesError::Process {
            message: sanitize_process_error(&error.to_string()),
        })?
        .into_iter()
        .filter_map(|document| project.store.get_document(&document.id).ok().flatten())
        .filter_map(|document| {
            (document.frontmatter.source_format.as_deref() == Some("apple_notes"))
                .then_some(document.frontmatter.source_path)
                .flatten()
        })
        .collect();
    for note in notes {
        let source_path = format!("apple-notes://{}/{}", note.account_id, note.source_id);
        if existing_source_ids.contains(&source_path) {
            report.skipped_existing += 1;
            continue;
        }
        if note.markdown.is_empty()
            && note
                .warnings
                .iter()
                .any(|warning| warning.contains("password-protected"))
        {
            report.skipped_locked += 1;
            continue;
        }
        let folder = safe_relative_components(&note.folder_path);
        let file_name = format!("{}.md", safe_component(&note.title));
        let mut path = PathBuf::from("Apple Notes Import");
        path.extend(folder);
        path.push(file_name);
        if project.root.join(&path).exists() {
            path.set_file_name(format!(
                "{}-{}.md",
                safe_component(&note.title),
                stable_suffix(&note.source_id)
            ));
        }

        let mut frontmatter = Frontmatter {
            id: Some(Uuid::new_v4()),
            title: Some(note.title.clone()),
            tags: vec!["apple-notes-import".into()],
            source_format: Some("apple_notes".into()),
            source_path: Some(source_path),
            imported_reference: false,
            ..Frontmatter::default()
        };
        frontmatter.metadata.insert(
            "apple_notes_id".into(),
            MetadataValue::String(note.source_id.clone()),
        );
        frontmatter.metadata.insert(
            "apple_notes_folder".into(),
            MetadataValue::String(note.folder_path.clone()),
        );
        frontmatter.metadata.insert(
            "apple_notes_created_at".into(),
            MetadataValue::String(note.created_at.clone()),
        );
        frontmatter.metadata.insert(
            "apple_notes_modified_at".into(),
            MetadataValue::String(note.modified_at.clone()),
        );
        frontmatter.metadata.insert(
            "apple_notes_shared".into(),
            MetadataValue::Bool(note.shared),
        );
        let encoded = toml::to_string(&frontmatter).map_err(|error| AppleNotesError::Process {
            message: sanitize_process_error(&error.to_string()),
        })?;
        let source = format!("+++\n{encoded}+++\n\n{}\n", note.markdown);
        project
            .create_document_source(&path, &source)
            .map_err(|error| AppleNotesError::Process {
                message: sanitize_process_error(&error.to_string()),
            })?;
        report.imported.push(ImportedAppleNote {
            source_id: note.source_id,
            path,
            warnings: note.warnings,
        });
    }
    Ok(report)
}

fn safe_relative_components(path: &str) -> Vec<String> {
    path.split('/')
        .filter(|part| !part.trim().is_empty())
        .map(safe_component)
        .collect()
}

fn safe_component(value: &str) -> String {
    let cleaned: String = value
        .chars()
        .map(|character| match character {
            '/' | ':' | '\\' | '\0' => '-',
            control if control.is_control() => ' ',
            other => other,
        })
        .collect();
    let cleaned = cleaned.trim().trim_matches('.').trim();
    if cleaned.is_empty() {
        "Untitled".into()
    } else {
        cleaned.chars().take(120).collect()
    }
}

fn stable_suffix(source_id: &str) -> String {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in source_id.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:08x}").chars().take(8).collect()
}

fn parse_catalog(bytes: &[u8]) -> Result<AppleNotesCatalog, AppleNotesError> {
    let catalog: AppleNotesCatalog =
        serde_json::from_slice(bytes).map_err(|_| AppleNotesError::MalformedResponse)?;
    if catalog.schema_version != 1 {
        return Err(AppleNotesError::UnsupportedSchema(catalog.schema_version));
    }
    Ok(catalog)
}

fn classify_process_failure(stderr: &str) -> AppleNotesError {
    let lower = stderr.to_ascii_lowercase();
    if lower.contains("-1743")
        || lower.contains("not authorized")
        || lower.contains("not permitted to send apple events")
    {
        AppleNotesError::PermissionDenied
    } else if lower.contains("application isn't running")
        || lower.contains("application can’t be found")
        || lower.contains("application can't be found")
    {
        AppleNotesError::NotesUnavailable
    } else {
        AppleNotesError::Process {
            message: sanitize_process_error(stderr),
        }
    }
}

/// Process diagnostics must not carry note content into application logs or UI.
fn sanitize_process_error(message: &str) -> String {
    let first_line = message.lines().next().unwrap_or("unknown error").trim();
    let bounded: String = first_line.chars().take(240).collect();
    if bounded.is_empty() {
        "unknown error".to_string()
    } else {
        bounded
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_nested_metadata_catalog() {
        let json = br#"{
          "schema_version": 1,
          "accounts": [{
            "id": "acct-1", "name": "iCloud",
            "folders": [{
              "id": "folder-1", "name": "Work", "path": "Work",
              "notes": [{
                "id": "note-1", "name": "Plan", "account_id": "acct-1",
                "folder_id": "folder-1", "folder_path": "Work",
                "created_at": "2026-01-01T00:00:00.000Z",
                "modified_at": "2026-01-02T00:00:00.000Z",
                "password_protected": false, "shared": true,
                "attachment_count": 2
              }],
              "folders": []
            }]
          }]
        }"#;

        let catalog = parse_catalog(json).unwrap();
        let note = &catalog.accounts[0].folders[0].notes[0];
        assert_eq!(note.id, "note-1");
        assert_eq!(note.folder_path, "Work");
        assert!(note.shared);
        assert_eq!(note.attachment_count, 2);
    }

    #[test]
    fn rejects_malformed_and_future_responses() {
        assert_eq!(
            parse_catalog(b"not json"),
            Err(AppleNotesError::MalformedResponse)
        );
        assert_eq!(
            parse_catalog(br#"{"schema_version":2,"accounts":[]}"#),
            Err(AppleNotesError::UnsupportedSchema(2))
        );
    }

    #[test]
    fn prepares_rich_note_and_surfaces_fidelity_warnings() {
        let prepared = prepare_note(AppleNoteExportRecord {
            id: "note-1".into(),
            name: "Plan".into(),
            account_id: "account-1".into(),
            folder_id: "folder-1".into(),
            folder_path: "Work / Plans".into(),
            created_at: "2026-01-01T00:00:00.000Z".into(),
            modified_at: "2026-01-02T00:00:00.000Z".into(),
            password_protected: false,
            shared: true,
            html: "<h1>Plan</h1><p><strong>Ship</strong> it.</p>".into(),
            plaintext: "Plan\nShip it.".into(),
            attachments: vec![AppleNoteAttachment {
                id: "attachment-1".into(),
                name: "brief.pdf".into(),
                content_id: String::new(),
                url: String::new(),
                created_at: String::new(),
                modified_at: String::new(),
            }],
        });
        assert_eq!(prepared.source_id, "note-1");
        assert!(prepared.markdown.contains("**Ship**"));
        assert_eq!(prepared.warnings.len(), 2);
    }

    #[test]
    fn locked_notes_are_prepared_as_skipped() {
        let prepared = prepare_note(AppleNoteExportRecord {
            id: "locked".into(),
            name: "Secret".into(),
            account_id: String::new(),
            folder_id: String::new(),
            folder_path: String::new(),
            created_at: String::new(),
            modified_at: String::new(),
            password_protected: true,
            shared: false,
            html: "private".into(),
            plaintext: "private".into(),
            attachments: Vec::new(),
        });
        assert!(prepared.markdown.is_empty());
        assert_eq!(prepared.warnings, ["password-protected note skipped"]);
    }

    #[test]
    fn classifies_permission_denial_without_exposing_multiline_output() {
        assert_eq!(
            classify_process_failure(
                "execution error: Not authorized to send Apple events to Notes. (-1743)\nprivate body"
            ),
            AppleNotesError::PermissionDenied
        );
        assert_eq!(
            classify_process_failure("unexpected failure\nprivate body"),
            AppleNotesError::Process {
                message: "unexpected failure".into()
            }
        );
    }

    #[cfg(target_os = "macos")]
    #[tokio::test]
    #[ignore = "prompts for macOS Automation permission"]
    async fn discovers_local_notes_metadata() {
        let catalog = discover_with_timeout(Duration::from_secs(60))
            .await
            .unwrap();
        assert_eq!(catalog.schema_version, 1);
    }
}
