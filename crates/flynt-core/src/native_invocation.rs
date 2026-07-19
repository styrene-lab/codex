//! Typed, platform-neutral contract for native Flynt invocation and capture.
//!
//! AppKit/UIKit adapters receive untrusted operating-system input and convert it
//! into these values. Flynt applications execute the resulting action; Omegon is
//! not required for deterministic navigation or capture.

use serde::{Deserialize, Serialize};
use std::path::{Component, Path};
use thiserror::Error;
use uuid::Uuid;

pub const INVOCATION_SCHEMA: &str = "flynt.invocation/v1";
pub const CAPTURE_SCHEMA: &str = "flynt.capture/v1";
pub const MAX_INVOCATION_URL_BYTES: usize = 16 * 1024;
pub const MAX_CAPTURE_TEXT_BYTES: usize = 1024 * 1024;
pub const MAX_CAPTURE_ITEMS: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlyntInstallation {
    Stable,
    Candidate,
    Dev,
}

impl FlyntInstallation {
    pub const fn scheme(self) -> &'static str {
        match self {
            Self::Stable => "flynt",
            Self::Candidate => "flynt-candidate",
            Self::Dev => "flynt-dev",
        }
    }

    pub fn from_scheme(value: &str) -> Result<Self, InvocationError> {
        match value {
            "flynt" | "flynt-note" => Ok(Self::Stable),
            "flynt-candidate" => Ok(Self::Candidate),
            "flynt-dev" => Ok(Self::Dev),
            _ => Err(InvocationError::UnsupportedScheme(value.to_string())),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct FlyntInvocation {
    pub schema: String,
    pub installation: FlyntInstallation,
    pub action: FlyntLinkAction,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FlyntLinkAction {
    OpenProject {
        project_id: String,
    },
    OpenDocument {
        project_id: String,
        document: DocumentReference,
    },
    OpenTask {
        project_id: String,
        task_id: Uuid,
    },
    OpenView {
        project_id: String,
        view: FlyntView,
    },
    Capture {
        project_id: Option<String>,
        request: CaptureRequest,
    },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "value", rename_all = "snake_case")]
pub enum DocumentReference {
    Id(Uuid),
    RelativePath(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FlyntView {
    Notes,
    Tasks,
    Graph,
    Search,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureRequest {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub item: CaptureItem,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CaptureEnvelope {
    pub schema: String,
    pub id: Uuid,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    pub items: Vec<CaptureItem>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CaptureItem {
    Text {
        value: String,
    },
    Url {
        value: String,
        title: Option<String>,
    },
    Asset {
        path: String,
        media_type: String,
    },
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum InvocationError {
    #[error("invocation URL is empty or too large")]
    InvalidSize,
    #[error("unsupported Flynt URL scheme: {0}")]
    UnsupportedScheme(String),
    #[error("invalid Flynt invocation URL")]
    InvalidUrl,
    #[error("unsupported Flynt action: {0}")]
    UnsupportedAction(String),
    #[error("missing required parameter: {0}")]
    MissingParameter(&'static str),
    #[error("invalid identifier: {0}")]
    InvalidIdentifier(&'static str),
    #[error("invalid percent encoding")]
    InvalidPercentEncoding,
    #[error("unsafe project-relative path")]
    UnsafeRelativePath,
    #[error("capture payload exceeds contract limits")]
    CaptureTooLarge,
    #[error("capture URL is not an allowed http(s) URL")]
    InvalidCaptureUrl,
    #[error("capture asset path or media type is invalid")]
    InvalidCaptureAsset,
}

impl FlyntInvocation {
    pub fn parse(input: &str) -> Result<Self, InvocationError> {
        if input.is_empty() || input.len() > MAX_INVOCATION_URL_BYTES {
            return Err(InvocationError::InvalidSize);
        }
        let (scheme, rest) = input.split_once("://").ok_or(InvocationError::InvalidUrl)?;
        let installation = FlyntInstallation::from_scheme(scheme)?;
        let (route, query) = rest.split_once('?').unwrap_or((rest, ""));
        let mut segments = route.split('/').filter(|segment| !segment.is_empty());
        let action_name = segments.next().ok_or(InvocationError::InvalidUrl)?;
        let route_value = segments.next();
        if segments.next().is_some() {
            return Err(InvocationError::InvalidUrl);
        }
        let query = parse_query(query)?;
        let project = || required_project(query_value(&query, "project"));

        let action = match action_name {
            "project" => FlyntLinkAction::OpenProject {
                project_id: validate_project_id(required(route_value, "project_id")?)?,
            },
            "note" => {
                let value = required(route_value, "document")?;
                let document = if let Ok(id) = Uuid::parse_str(value) {
                    DocumentReference::Id(id)
                } else {
                    DocumentReference::RelativePath(validate_relative_path(&percent_decode(
                        value,
                    )?)?)
                };
                FlyntLinkAction::OpenDocument {
                    project_id: project()?,
                    document,
                }
            }
            "task" => FlyntLinkAction::OpenTask {
                project_id: project()?,
                task_id: Uuid::parse_str(required(route_value, "task_id")?)
                    .map_err(|_| InvocationError::InvalidIdentifier("task_id"))?,
            },
            "view" => FlyntLinkAction::OpenView {
                project_id: project()?,
                view: parse_view(required(route_value, "view")?)?,
            },
            "capture" => {
                if route_value.is_some() {
                    return Err(InvocationError::InvalidUrl);
                }
                let title = query_value(&query, "title").map(str::to_string);
                let text = query_value(&query, "text");
                let url = query_value(&query, "url");
                let item = match (text, url) {
                    (Some(value), None) => {
                        validate_capture_text(value)?;
                        CaptureItem::Text {
                            value: value.into(),
                        }
                    }
                    (None, Some(value)) => {
                        validate_web_url(value)?;
                        CaptureItem::Url {
                            value: value.into(),
                            title: title.clone(),
                        }
                    }
                    _ => return Err(InvocationError::MissingParameter("text or url")),
                };
                FlyntLinkAction::Capture {
                    project_id: query_value(&query, "project")
                        .map(validate_project_id)
                        .transpose()?,
                    request: CaptureRequest { title, item },
                }
            }
            other => return Err(InvocationError::UnsupportedAction(other.to_string())),
        };

        Ok(Self {
            schema: INVOCATION_SCHEMA.into(),
            installation,
            action,
        })
    }
}

impl CaptureEnvelope {
    pub fn validate(&self) -> Result<(), InvocationError> {
        if self.schema != CAPTURE_SCHEMA
            || self.items.is_empty()
            || self.items.len() > MAX_CAPTURE_ITEMS
        {
            return Err(InvocationError::CaptureTooLarge);
        }
        for item in &self.items {
            match item {
                CaptureItem::Text { value } => validate_capture_text(value)?,
                CaptureItem::Url { value, .. } => validate_web_url(value)?,
                CaptureItem::Asset { path, media_type } => {
                    validate_relative_path(path)
                        .map_err(|_| InvocationError::InvalidCaptureAsset)?;
                    if media_type.is_empty() || media_type.len() > 255 || !media_type.contains('/')
                    {
                        return Err(InvocationError::InvalidCaptureAsset);
                    }
                }
            }
        }
        Ok(())
    }
}

fn required<'a>(value: Option<&'a str>, name: &'static str) -> Result<&'a str, InvocationError> {
    value
        .filter(|v| !v.is_empty())
        .ok_or(InvocationError::MissingParameter(name))
}

fn required_project(value: Option<&str>) -> Result<String, InvocationError> {
    validate_project_id(required(value, "project")?)
}

fn validate_project_id(value: &str) -> Result<String, InvocationError> {
    if value.is_empty() || value.len() > 256 || value.contains(['/', '\\', '\0']) {
        return Err(InvocationError::InvalidIdentifier("project"));
    }
    Ok(value.to_string())
}

fn validate_relative_path(value: &str) -> Result<String, InvocationError> {
    let path = Path::new(value);
    if value.is_empty() || value.len() > 4096 || path.is_absolute() || value.contains('\0') {
        return Err(InvocationError::UnsafeRelativePath);
    }
    if path
        .components()
        .any(|part| !matches!(part, Component::Normal(_)))
    {
        return Err(InvocationError::UnsafeRelativePath);
    }
    Ok(value.to_string())
}

fn parse_view(value: &str) -> Result<FlyntView, InvocationError> {
    match value {
        "notes" => Ok(FlyntView::Notes),
        "tasks" => Ok(FlyntView::Tasks),
        "graph" => Ok(FlyntView::Graph),
        "search" => Ok(FlyntView::Search),
        _ => Err(InvocationError::UnsupportedAction(format!("view/{value}"))),
    }
}

fn validate_capture_text(value: &str) -> Result<(), InvocationError> {
    if value.is_empty() || value.len() > MAX_CAPTURE_TEXT_BYTES || value.contains('\0') {
        return Err(InvocationError::CaptureTooLarge);
    }
    Ok(())
}

fn validate_web_url(value: &str) -> Result<(), InvocationError> {
    if value.len() > 8192
        || !(value.starts_with("https://") || value.starts_with("http://"))
        || value.contains(['\0', '\n', '\r'])
    {
        return Err(InvocationError::InvalidCaptureUrl);
    }
    Ok(())
}

fn parse_query(value: &str) -> Result<Vec<(String, String)>, InvocationError> {
    if value.is_empty() {
        return Ok(Vec::new());
    }
    value
        .split('&')
        .map(|pair| {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            Ok((percent_decode(key)?, percent_decode(value)?))
        })
        .collect()
}

fn query_value<'a>(query: &'a [(String, String)], key: &str) -> Option<&'a str> {
    query
        .iter()
        .find(|(candidate, _)| candidate == key)
        .map(|(_, value)| value.as_str())
}

fn percent_decode(value: &str) -> Result<String, InvocationError> {
    let bytes = value.as_bytes();
    let mut output = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        match bytes[index] {
            b'%' if index + 2 < bytes.len() => {
                let high = hex(bytes[index + 1])?;
                let low = hex(bytes[index + 2])?;
                output.push((high << 4) | low);
                index += 3;
            }
            b'%' => return Err(InvocationError::InvalidPercentEncoding),
            b'+' => {
                output.push(b' ');
                index += 1;
            }
            byte => {
                output.push(byte);
                index += 1;
            }
        }
    }
    String::from_utf8(output).map_err(|_| InvocationError::InvalidPercentEncoding)
}

fn hex(value: u8) -> Result<u8, InvocationError> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        b'A'..=b'F' => Ok(value - b'A' + 10),
        _ => Err(InvocationError::InvalidPercentEncoding),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PROJECT: &str = "7dc879c5-fd9c-4b2c-a41b-f38b50f265b2";

    #[test]
    fn parses_identity_specific_note_links() {
        let link = FlyntInvocation::parse(&format!(
            "flynt-dev://note/notes%2Ftoday.md?project={PROJECT}"
        ))
        .unwrap();
        assert_eq!(link.installation, FlyntInstallation::Dev);
        assert_eq!(
            link.action,
            FlyntLinkAction::OpenDocument {
                project_id: PROJECT.into(),
                document: DocumentReference::RelativePath("notes/today.md".into())
            }
        );
    }

    #[test]
    fn stable_accepts_legacy_scheme_only() {
        let link =
            FlyntInvocation::parse(&format!("flynt-note://view/graph?project={PROJECT}")).unwrap();
        assert_eq!(link.installation, FlyntInstallation::Stable);
    }

    #[test]
    fn rejects_absolute_and_traversal_paths() {
        for path in [
            "%2Fetc%2Fpasswd",
            "..%2Fsecret.md",
            "notes%2F..%2Fsecret.md",
        ] {
            assert_eq!(
                FlyntInvocation::parse(&format!("flynt://note/{path}?project={PROJECT}")),
                Err(InvocationError::UnsafeRelativePath)
            );
        }
    }

    #[test]
    fn capture_requires_one_bounded_payload() {
        let invocation = FlyntInvocation::parse(
            "flynt://capture?title=Read&url=https%3A%2F%2Fexample.com%2Fpost",
        )
        .unwrap();
        assert!(matches!(invocation.action, FlyntLinkAction::Capture { .. }));
        assert!(FlyntInvocation::parse("flynt://capture?text=a&url=https%3A%2F%2Fx.test").is_err());
        assert!(FlyntInvocation::parse("flynt://capture?url=file%3A%2F%2Fetc%2Fpasswd").is_err());
    }

    #[test]
    fn validates_neutral_capture_envelopes() {
        let envelope = CaptureEnvelope {
            schema: CAPTURE_SCHEMA.into(),
            id: Uuid::new_v4(),
            created_at: "2026-07-19T12:00:00Z".into(),
            title: None,
            items: vec![CaptureItem::Asset {
                path: "assets/image.png".into(),
                media_type: "image/png".into(),
            }],
        };
        assert_eq!(envelope.validate(), Ok(()));
    }
}
