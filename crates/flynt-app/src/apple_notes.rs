//! macOS Apple Notes metadata discovery through the supported Notes scripting API.
//!
//! This module deliberately exposes summaries only. Importing bodies and attachments
//! is a separate step so opening the picker does not traverse private content.

use serde::{Deserialize, Serialize};
use std::{path::Path, process::Stdio, time::Duration};
use thiserror::Error;

const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

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

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
pub struct AppleNotesCatalog {
    pub schema_version: u32,
    pub accounts: Vec<AppleNotesAccount>,
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
