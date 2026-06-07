//! ACP (Agent Client Protocol) client for communicating with Omegon.
//!
//! Spawns `omegon acp` as a child process and communicates via structured
//! JSON-RPC over stdio. Streams text deltas, tool calls, slash commands,
//! config options, and auth events back to the UI through a channel.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

use crate::omegon_cli_contract::OmegonCliContract;
use agent_client_protocol::schema::{
    AgentNotification, CancelNotification, ClientCapabilities, ClientRequest, ContentBlock,
    ExtNotification, ExtRequest, InitializeRequest, ListSessionsRequest, LoadSessionRequest,
    NewSessionRequest, PermissionOption, PermissionOptionId, PermissionOptionKind, PromptRequest,
    ProtocolVersion, RequestPermissionOutcome, RequestPermissionRequest, RequestPermissionResponse,
    SelectedPermissionOutcome, SessionConfigId, SessionConfigKind, SessionConfigOption,
    SessionConfigSelectOptions, SessionConfigValueId, SessionId, SessionNotification,
    SessionUpdate, SetSessionConfigOptionRequest, TextContent,
};
use agent_client_protocol::{Agent, ConnectionTo};
use anyhow::Result;
use tokio::sync::oneshot;

/// Events flowing from the ACP session to the UI.
#[derive(Debug, Clone)]
pub enum AcpEvent {
    /// Incremental text from the agent's response.
    TextDelta(String),
    /// Agent is thinking / internal reasoning.
    ThoughtDelta(String),
    /// A new tool call started.
    ToolCallStarted {
        id: String,
        title: String,
        kind: String,
        /// Raw input args from the agent — used to render call metadata
        /// alongside the tool name (None if the agent didn't supply any).
        args: Option<serde_json::Value>,
    },
    /// A tool call status changed.
    ToolCallUpdated {
        id: String,
        status: String,
        title: Option<String>,
        /// Text output from the tool, if any. Concatenated from text content blocks.
        output: Option<String>,
        /// Raw output/details emitted by the tool update. HostAction metadata is carried here
        /// by Omegon 0.24 so Flynt can render review/outcome cards without scraping prose.
        raw_output: Option<serde_json::Value>,
        terminal_ids: Vec<String>,
    },
    PermissionRequested(PendingPermissionRequest),
    /// ACP initialize metadata observed from the agent process.
    DeploymentMetadata(serde_json::Value),
    /// Available slash commands changed.
    CommandsAvailable(Vec<SlashCommand>),
    /// Config options changed (model, thinking, posture, etc).
    ConfigChanged(Vec<ConfigOption>),
    /// The agent's execution plan for the current turn. Replaces any
    /// previous plan — the agent emits the full list with each update.
    PlanUpdated(Vec<PlanItem>),
    /// Session metadata changed — typically the title once the agent
    /// has derived one from the first prompt.
    SessionTitleChanged(Option<String>),
    /// The prompt completed.
    Done,
    /// Structured provider retry telemetry from Omegon.
    ProviderRetry(serde_json::Value),
    /// Structured provider terminal failure telemetry from Omegon.
    ProviderFailure(serde_json::Value),
    /// Structured turn cancellation telemetry from Omegon.
    TurnCancelled(serde_json::Value),
    /// An error occurred.
    Error(String),
}

/// One entry in the agent's execution plan. Mirrors `agent_client_protocol::PlanEntry`
/// but stripped of meta and reduced to a UI-friendly shape.
#[derive(Debug, Clone, PartialEq)]
pub struct PlanItem {
    pub content: String,
    pub status: PlanStatus,
    pub priority: PlanPriority,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanStatus {
    Pending,
    InProgress,
    Completed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlanPriority {
    Low,
    Medium,
    High,
}

/// A slash command advertised by the agent.
#[derive(Debug, Clone, PartialEq)]
pub struct SlashCommand {
    pub name: String,
    pub description: String,
}

/// A config option (select dropdown) from the agent.
#[derive(Debug, Clone, PartialEq)]
pub struct ConfigOption {
    pub id: String,
    pub name: String,
    pub current_value: String,
    pub options: Vec<ConfigValue>,
}

/// A single selectable value in a config option.
#[derive(Debug, Clone, PartialEq)]
pub struct ConfigValue {
    pub value: String,
    pub name: String,
}

#[derive(Debug, Clone)]
pub struct PendingPermissionRequest {
    pub request_id: String,
    pub title: String,
    pub kind: String,
    pub raw_input: Option<serde_json::Value>,
    pub options: Vec<PermissionOptionView>,
    responder: std::sync::Arc<std::sync::Mutex<Option<oneshot::Sender<PermissionDecision>>>>,
}

impl PendingPermissionRequest {
    pub fn respond(&self, decision: PermissionDecision) {
        if let Some(tx) = self.responder.lock().unwrap().take() {
            let _ = tx.send(decision);
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PermissionOptionView {
    pub option_id: String,
    pub name: String,
    pub kind: PermissionOptionKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PermissionDecision {
    Approve,
    Reject,
}

/// Read-only Omegon ACP runtime capabilities projection.
#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct OmegonRuntimeCapabilities {
    #[serde(default)]
    pub capabilities: serde_json::Value,
    #[serde(default)]
    pub features: serde_json::Map<String, serde_json::Value>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

impl OmegonRuntimeCapabilities {
    pub fn plan_tasks_contract(&self) -> Option<OmegonPlanTasksContract> {
        self.features
            .get("plan_tasks_contract")
            .cloned()
            .and_then(|value| serde_json::from_value(value).ok())
    }
}

/// Machine-readable plan/task compatibility contract from `_runtime/capabilities`.
#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct OmegonPlanTasksContract {
    #[serde(default)]
    pub compatibility: Vec<String>,
    #[serde(default)]
    pub stable_id: bool,
    #[serde(default)]
    pub revision: bool,
    #[serde(default)]
    pub durable_bind: bool,
    #[serde(default)]
    pub structured_errors: bool,
    #[serde(default)]
    pub pagination: bool,
    #[serde(default)]
    pub filtering: bool,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// Source artifact information for projected Omegon tasks.
#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct OmegonTaskSourceRef {
    pub kind: String,
    #[serde(default)]
    pub path: Option<String>,
    #[serde(default)]
    pub anchor: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// Summary entry returned by Omegon `_plans/list`.
#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct OmegonPlanSummary {
    pub id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub revision: Option<String>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// Detailed read-only projection returned by Omegon `_plans/show`.
#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct OmegonPlanDetail {
    pub id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub revision: Option<String>,
    #[serde(default)]
    pub tasks: Vec<OmegonTaskSummary>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// Summary entry returned by Omegon `_tasks/list`.
#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct OmegonTaskSummary {
    pub id: String,
    #[serde(default)]
    pub stable_id: Option<String>,
    #[serde(default)]
    pub source: Option<OmegonTaskSourceRef>,
    #[serde(default)]
    pub plan_id: Option<String>,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub revision: Option<String>,
    #[serde(default)]
    pub writable: Option<bool>,
    #[serde(default)]
    pub supported_mutations: Vec<String>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// Detailed read-only projection returned by Omegon `_tasks/show`.
#[derive(Debug, Clone, PartialEq, serde::Deserialize, serde::Serialize)]
pub struct OmegonTaskDetail {
    pub id: String,
    #[serde(default)]
    pub stable_id: Option<String>,
    #[serde(default)]
    pub source: Option<OmegonTaskSourceRef>,
    #[serde(default)]
    pub plan_id: Option<String>,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub revision: Option<String>,
    #[serde(default)]
    pub writable: Option<bool>,
    #[serde(default)]
    pub supported_mutations: Vec<String>,
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

fn unwrap_projection_payload(value: serde_json::Value, key: &str) -> serde_json::Value {
    match value {
        serde_json::Value::Object(mut object) => object
            .remove(key)
            .or_else(|| object.remove("result").and_then(|mut result| match result {
                serde_json::Value::Object(ref mut result_object) => result_object.remove(key),
                other => Some(other),
            }))
            .unwrap_or(serde_json::Value::Object(object)),
        other => other,
    }
}

fn parse_projection<T: serde::de::DeserializeOwned>(
    value: serde_json::Value,
    key: &str,
) -> Result<T> {
    Ok(serde_json::from_value(unwrap_projection_payload(value, key))?)
}

/// Extract config options from ACP SessionConfigOption list.
fn extract_config_options(opts: &[SessionConfigOption]) -> Vec<ConfigOption> {
    opts.iter()
        .filter_map(|opt| {
            if let SessionConfigKind::Select(sel) = &opt.kind {
                let values = match &sel.options {
                    SessionConfigSelectOptions::Ungrouped(list) => list
                        .iter()
                        .map(|o| ConfigValue {
                            value: o.value.to_string(),
                            name: o.name.clone(),
                        })
                        .collect(),
                    SessionConfigSelectOptions::Grouped(groups) => groups
                        .iter()
                        .flat_map(|g| {
                            g.options.iter().map(|o| ConfigValue {
                                value: o.value.to_string(),
                                name: o.name.clone(),
                            })
                        })
                        .collect(),
                    _ => return None,
                };
                Some(ConfigOption {
                    id: opt.id.to_string(),
                    name: opt.name.clone(),
                    current_value: sel.current_value.to_string(),
                    options: values,
                })
            } else {
                None
            }
        })
        .collect()
}

type EventSender = Arc<Mutex<std::sync::mpsc::Sender<AcpEvent>>>;

fn allow_response(options: &[PermissionOption]) -> Option<RequestPermissionResponse> {
    choose_option(
        options,
        &[
            PermissionOptionKind::AllowOnce,
            PermissionOptionKind::AllowAlways,
        ],
    )
    .map(selected_response)
}

fn reject_response(options: &[PermissionOption]) -> RequestPermissionResponse {
    choose_option(
        options,
        &[
            PermissionOptionKind::RejectOnce,
            PermissionOptionKind::RejectAlways,
        ],
    )
    .map(selected_response)
    .unwrap_or_else(|| RequestPermissionResponse::new(RequestPermissionOutcome::Cancelled))
}

fn choose_option(
    options: &[PermissionOption],
    preferred: &[PermissionOptionKind],
) -> Option<PermissionOptionId> {
    preferred.iter().find_map(|kind| {
        options
            .iter()
            .find(|option| option.kind == *kind)
            .map(|option| option.option_id.clone())
    })
}

fn selected_response(option_id: PermissionOptionId) -> RequestPermissionResponse {
    RequestPermissionResponse::new(RequestPermissionOutcome::Selected(
        SelectedPermissionOutcome::new(option_id),
    ))
}

fn handle_ext_notification(tx: &EventSender, notification: ExtNotification) {
    let value: serde_json::Value = serde_json::from_str(notification.params.get())
        .unwrap_or_else(|_| serde_json::json!({ "raw": notification.params.get() }));
    let event = match notification.method.as_ref() {
        "provider/retry" | "_provider/retry" => AcpEvent::ProviderRetry(value),
        "provider/failure" | "_provider/failure" => AcpEvent::ProviderFailure(value),
        "turn/cancelled" | "_turn/cancelled" => AcpEvent::TurnCancelled(value),
        other => {
            tracing::debug!(method = other, "Ignoring ACP extension notification");
            return;
        }
    };
    let _ = tx.lock().unwrap().send(event);
}

fn handle_session_notification(tx: &EventSender, args: SessionNotification) {
    let tx_ref = tx.lock().unwrap();
    match args.update {
        SessionUpdate::AgentMessageChunk(chunk) => {
            if let ContentBlock::Text(text) = chunk.content {
                let _ = tx_ref.send(AcpEvent::TextDelta(text.text));
            }
        }
        SessionUpdate::AgentThoughtChunk(chunk) => {
            if let ContentBlock::Text(text) = chunk.content {
                let _ = tx_ref.send(AcpEvent::ThoughtDelta(text.text));
            }
        }
        SessionUpdate::ToolCall(tc) => {
            let _ = tx_ref.send(AcpEvent::ToolCallStarted {
                id: tc.tool_call_id.to_string(),
                title: tc.title,
                kind: format!("{:?}", tc.kind),
                args: tc.raw_input,
            });
        }
        SessionUpdate::ToolCallUpdate(update) => {
            let mut terminal_ids = Vec::new();
            let output = update
                .fields
                .content
                .as_ref()
                .map(|blocks| {
                    let mut out = String::new();
                    for block in blocks {
                        match block {
                            agent_client_protocol::schema::ToolCallContent::Content(c) => {
                                if let ContentBlock::Text(t) = &c.content {
                                    if !out.is_empty() {
                                        out.push('\n');
                                    }
                                    out.push_str(&t.text);
                                }
                            }
                            agent_client_protocol::schema::ToolCallContent::Terminal(t) => {
                                terminal_ids.push(t.terminal_id.to_string());
                            }
                            _ => {}
                        }
                    }
                    out
                })
                .filter(|s| !s.is_empty());
            let _ = tx_ref.send(AcpEvent::ToolCallUpdated {
                id: update.tool_call_id.to_string(),
                status: update
                    .fields
                    .status
                    .map(|s| format!("{s:?}"))
                    .unwrap_or_default(),
                title: update.fields.title,
                output,
                raw_output: update.fields.raw_output,
                terminal_ids,
            });
        }
        SessionUpdate::Plan(plan) => {
            let items = plan
                .entries
                .into_iter()
                .map(|e| PlanItem {
                    content: e.content,
                    status: match e.status {
                        agent_client_protocol::schema::PlanEntryStatus::Pending => {
                            PlanStatus::Pending
                        }
                        agent_client_protocol::schema::PlanEntryStatus::InProgress => {
                            PlanStatus::InProgress
                        }
                        agent_client_protocol::schema::PlanEntryStatus::Completed => {
                            PlanStatus::Completed
                        }
                        _ => PlanStatus::Pending,
                    },
                    priority: match e.priority {
                        agent_client_protocol::schema::PlanEntryPriority::High => {
                            PlanPriority::High
                        }
                        agent_client_protocol::schema::PlanEntryPriority::Medium => {
                            PlanPriority::Medium
                        }
                        agent_client_protocol::schema::PlanEntryPriority::Low => PlanPriority::Low,
                        _ => PlanPriority::Medium,
                    },
                })
                .collect();
            let _ = tx_ref.send(AcpEvent::PlanUpdated(items));
        }
        SessionUpdate::SessionInfoUpdate(info) => {
            let title = match serde_json::to_value(&info.title).ok() {
                Some(serde_json::Value::String(s)) => Some(Some(s)),
                Some(serde_json::Value::Null) => Some(None),
                _ => None,
            };
            if let Some(t) = title {
                let _ = tx_ref.send(AcpEvent::SessionTitleChanged(t));
            }
            if let Some(meta) = info.meta.as_ref().and_then(|meta| meta.get("flynt")) {
                let _ = tx_ref.send(AcpEvent::DeploymentMetadata(meta.clone()));
            }
        }
        SessionUpdate::AvailableCommandsUpdate(cmds) => {
            let commands = cmds
                .available_commands
                .into_iter()
                .map(|c| SlashCommand {
                    name: c.name,
                    description: c.description,
                })
                .collect();
            let _ = tx_ref.send(AcpEvent::CommandsAvailable(commands));
        }
        SessionUpdate::ConfigOptionUpdate(update) => {
            let opts = extract_config_options(&update.config_options);
            if !opts.is_empty() {
                let _ = tx_ref.send(AcpEvent::ConfigChanged(opts));
            }
        }
        _ => {}
    }
}

async fn handle_request_permission(
    tx: EventSender,
    args: RequestPermissionRequest,
    responder: agent_client_protocol::Responder<RequestPermissionResponse>,
) -> agent_client_protocol::Result<()> {
    let (decision_tx, decision_rx) = oneshot::channel();
    let options = args
        .options
        .iter()
        .map(|option| PermissionOptionView {
            option_id: option.option_id.to_string(),
            name: option.name.clone(),
            kind: option.kind,
        })
        .collect::<Vec<_>>();
    let fallback = reject_response(&args.options);
    let request = PendingPermissionRequest {
        request_id: args.tool_call.tool_call_id.to_string(),
        title: args
            .tool_call
            .fields
            .title
            .clone()
            .unwrap_or_else(|| "Permission request".to_string()),
        kind: args
            .tool_call
            .fields
            .kind
            .as_ref()
            .map(|kind| format!("{kind:?}"))
            .unwrap_or_else(|| "Tool".to_string()),
        raw_input: args.tool_call.fields.raw_input.clone(),
        options,
        responder: std::sync::Arc::new(std::sync::Mutex::new(Some(decision_tx))),
    };
    if tx
        .lock()
        .unwrap()
        .send(AcpEvent::PermissionRequested(request))
        .is_err()
    {
        return responder.respond(fallback);
    }
    match decision_rx.await {
        Ok(PermissionDecision::Approve) => {
            responder.respond(allow_response(&args.options).unwrap_or(fallback))
        }
        Ok(PermissionDecision::Reject) | Err(_) => responder.respond(fallback),
    }
}

/// A live ACP session connected to an Omegon child process.
pub struct AcpSession {
    conn: Rc<ConnectionTo<Agent>>,
    session_id: Rc<RefCell<SessionId>>,
    tx: std::sync::mpsc::Sender<AcpEvent>,
    #[allow(dead_code)]
    auth_method_id: Option<String>,
}

impl AcpSession {
    /// Spawn `omegon acp` and perform the ACP handshake.
    pub async fn connect(
        omegon_binary: PathBuf,
        cwd: PathBuf,
        agent_id: Option<String>,
    ) -> Result<(Self, std::sync::mpsc::Receiver<AcpEvent>)> {
        let (tx, rx) = std::sync::mpsc::channel();
        let done_tx = tx.clone();

        let contract = OmegonCliContract::current();
        let mut server =
            agent_client_protocol::schema::McpServerStdio::new("omegon", omegon_binary.clone());
        server.args = contract.acp_args(&cwd, agent_id.as_deref());
        server
            .env
            .push(agent_client_protocol::schema::EnvVariable::new(
                "FLYNT_PROJECT",
                cwd.to_string_lossy().to_string(),
            ));
        server
            .env
            .push(agent_client_protocol::schema::EnvVariable::new(
                "OMEGON_CHILD_ENABLED_EXTENSIONS",
                "flynt",
            ));
        let agent = agent_client_protocol::AcpAgent::new(
            agent_client_protocol::schema::McpServer::Stdio(server),
        );

        let event_tx: EventSender = Arc::new(Mutex::new(tx));
        let (conn_tx, conn_rx) = oneshot::channel();
        let io_err_tx = done_tx.clone();
        let permission_tx = event_tx.clone();
        let session_tx = event_tx.clone();
        let ext_tx = event_tx.clone();
        tokio::spawn(async move {
            let result = agent_client_protocol::Client
                .builder()
                .on_receive_request(
                    async move |request: RequestPermissionRequest, responder, _cx| {
                        handle_request_permission(permission_tx.clone(), request, responder).await
                    },
                    agent_client_protocol::on_receive_request!(),
                )
                .on_receive_notification(
                    async move |notification: AgentNotification, _cx| {
                        match notification {
                            AgentNotification::SessionNotification(notification) => {
                                handle_session_notification(&session_tx, notification);
                            }
                            AgentNotification::ExtNotification(notification) => {
                                handle_ext_notification(&ext_tx, notification);
                            }
                            _ => {}
                        }
                        Ok(())
                    },
                    agent_client_protocol::on_receive_notification!(),
                )
                .connect_with(agent, async move |connection: ConnectionTo<Agent>| {
                    let _ = conn_tx.send(connection.clone());
                    futures::future::pending::<agent_client_protocol::Result<()>>().await
                })
                .await;
            if let Err(e) = result {
                tracing::error!("ACP I/O error: {e}");
                let _ = io_err_tx.send(AcpEvent::Error(format!("ACP transport disconnected: {e}")));
            } else {
                tracing::warn!("ACP I/O task ended");
                let _ = io_err_tx.send(AcpEvent::Error("ACP transport disconnected".into()));
            }
        });

        let conn = conn_rx
            .await
            .map_err(|_| anyhow::anyhow!("ACP connection failed before initialization"))?;
        let conn = Rc::new(conn);

        // Initialize
        let init_resp = conn
            .send_request(
                InitializeRequest::new(ProtocolVersion::V1)
                    .client_capabilities(ClientCapabilities::new().terminal(true)),
            )
            .block_task()
            .await
            .map_err(|e| anyhow::anyhow!("ACP init failed: {e}"))?;

        if let Some(meta) = init_resp.meta.clone() {
            if let Some(flynt) = meta.get("flynt") {
                let _ = done_tx.send(AcpEvent::DeploymentMetadata(flynt.clone()));
            }
        }

        let auth_method_id = init_resp.auth_methods.first().map(|m| m.id().to_string());

        // Create session
        let session_resp = conn
            .send_request(NewSessionRequest::new(&cwd))
            .block_task()
            .await
            .map_err(|e| anyhow::anyhow!("ACP session failed: {e}"))?;

        // Send initial config options
        if let Some(opts) = &session_resp.config_options {
            let config = extract_config_options(opts);
            if !config.is_empty() {
                let _ = done_tx.send(AcpEvent::ConfigChanged(config));
            }
        }

        Ok((
            Self {
                conn,
                session_id: Rc::new(RefCell::new(session_resp.session_id)),
                tx: done_tx,
                auth_method_id,
            },
            rx,
        ))
    }

    /// Trigger OAuth login by spawning `omegon auth login [provider]`.
    /// This opens the browser for the OAuth flow and resolves only after the CLI exits.
    pub async fn login(&self, omegon_binary: &PathBuf, provider: &str) -> Result<String> {
        let provider = if provider.is_empty() {
            "anthropic"
        } else {
            provider
        };
        let _ = self
            .tx
            .send(AcpEvent::TextDelta(format!("Opening {provider} login…\n")));

        let contract = OmegonCliContract::current();
        let output = tokio::process::Command::new(omegon_binary)
            .args(contract.auth_login_args(provider))
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .output()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to run omegon auth login: {e}"))?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            let msg = if stdout.trim().is_empty() {
                format!("Logged in to {provider}.")
            } else {
                stdout.trim().to_string()
            };
            let _ = self.tx.send(AcpEvent::TextDelta(msg.clone()));
            Ok(msg)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            let msg = if !stderr.trim().is_empty() {
                stderr.trim().to_string()
            } else if !stdout.trim().is_empty() {
                stdout.trim().to_string()
            } else {
                format!("Login to {provider} failed (exit code {})", output.status)
            };
            let _ = self.tx.send(AcpEvent::Error(msg.clone()));
            Err(anyhow::anyhow!(msg))
        }
    }

    pub fn current_session_id(&self) -> SessionId {
        self.session_id.borrow().clone()
    }

    pub async fn list_sessions(&self, cwd: Option<PathBuf>) -> Result<serde_json::Value> {
        let req = ListSessionsRequest::new().cwd(cwd);
        let resp = self.conn.send_request(req).block_task().await?;
        Ok(serde_json::to_value(resp)?)
    }

    pub async fn load_session(
        &self,
        session_id: SessionId,
        cwd: PathBuf,
    ) -> Result<serde_json::Value> {
        let req = LoadSessionRequest::new(session_id.clone(), cwd);
        let resp = self.conn.send_request(req).block_task().await?;
        *self.session_id.borrow_mut() = session_id;
        Ok(serde_json::to_value(resp)?)
    }

    pub async fn new_session(&self, cwd: PathBuf) -> Result<serde_json::Value> {
        let resp = self
            .conn
            .send_request(NewSessionRequest::new(cwd))
            .block_task()
            .await?;
        *self.session_id.borrow_mut() = resp.session_id.clone();
        Ok(serde_json::to_value(resp)?)
    }

    pub fn cancel_current_turn(&self) -> Result<()> {
        self.conn
            .send_notification(CancelNotification::new(self.current_session_id()))?;
        Ok(())
    }

    /// Send a user prompt.
    pub fn prompt(&self, text: &str) {
        tracing::info!(
            "AcpSession::prompt sending to Omegon ({} chars)",
            text.len()
        );
        let req = PromptRequest::new(
            self.session_id.borrow().clone(),
            vec![ContentBlock::Text(TextContent::new(text))],
        );
        let conn = self.conn.clone();
        let tx = self.tx.clone();
        dioxus::prelude::spawn(async move {
            match conn.send_request(req).block_task().await {
                Ok(_) => {
                    tracing::info!("AcpSession::prompt completed");
                    let _ = tx.send(AcpEvent::Done);
                }
                Err(e) => {
                    tracing::error!("AcpSession::prompt failed: {e}");
                    let _ = tx.send(AcpEvent::Error(format!("{e}")));
                }
            }
        });
    }

    /// Change a config option (model, thinking, posture).
    pub async fn set_config(&self, config_id: &str, value: &str) {
        let req = SetSessionConfigOptionRequest::new(
            self.session_id.borrow().clone(),
            SessionConfigId::new(config_id),
            SessionConfigValueId::new(value),
        );
        if let Err(e) = self.conn.send_request(req).block_task().await {
            let _ = self
                .tx
                .send(AcpEvent::Error(format!("Config change failed: {e}")));
        }
    }

    // ── Extension management ──────────────────────────────────────────

    async fn ext_call(&self, method: &str, params: serde_json::Value) -> Result<serde_json::Value> {
        let raw_params = serde_json::value::RawValue::from_string(serde_json::to_string(&params)?)?;
        let wire_method = if method.starts_with('_') {
            method.to_string()
        } else {
            format!("_{method}")
        };
        let req = ExtRequest::new(wire_method, raw_params.into());
        let value = self
            .conn
            .send_request(ClientRequest::ExtMethodRequest(req))
            .block_task()
            .await
            .map_err(|e| anyhow::anyhow!("ext_method {method} failed: {e}"))?;
        if let Some(err) = value["error"].as_str() {
            anyhow::bail!("{err}");
        }
        Ok(value)
    }

    /// List all installed extensions with config schema, current values, and secret status.
    pub async fn extensions_list(&self) -> Result<serde_json::Value> {
        self.ext_call("extensions/list", serde_json::json!({}))
            .await
    }

    /// Set a config value for an extension.
    pub async fn extensions_config_set(
        &self,
        extension: &str,
        key: &str,
        value: &str,
    ) -> Result<serde_json::Value> {
        self.ext_call(
            "extensions/config_set",
            serde_json::json!({
                "extension": extension,
                "key": key,
                "value": value,
            }),
        )
        .await
    }

    /// Store a secret in the OS keychain for an extension.
    pub async fn extensions_secret_set(
        &self,
        extension: &str,
        name: &str,
        value: &str,
    ) -> Result<serde_json::Value> {
        self.ext_call(
            "extensions/secret_set",
            serde_json::json!({
                "extension": extension,
                "name": name,
                "value": value,
            }),
        )
        .await
    }

    /// Delete a secret from the keychain.
    pub async fn extensions_secret_delete(&self, name: &str) -> Result<serde_json::Value> {
        self.ext_call(
            "extensions/secret_delete",
            serde_json::json!({
                "name": name,
            }),
        )
        .await
    }

    /// Enable an extension.
    pub async fn extensions_enable(&self, extension: &str) -> Result<serde_json::Value> {
        self.ext_call(
            "extensions/enable",
            serde_json::json!({
                "extension": extension,
            }),
        )
        .await
    }

    /// Disable an extension.
    pub async fn extensions_disable(&self, extension: &str) -> Result<serde_json::Value> {
        self.ext_call(
            "extensions/disable",
            serde_json::json!({
                "extension": extension,
            }),
        )
        .await
    }

    /// Install an extension from a local path, git URL, or tarball URI.
    pub async fn extensions_install(&self, uri: &str) -> Result<serde_json::Value> {
        self.ext_call(
            "extensions/install",
            serde_json::json!({
                "uri": uri,
            }),
        )
        .await
    }

    /// Remove an installed extension.
    pub async fn extensions_remove(&self, extension: &str) -> Result<serde_json::Value> {
        self.ext_call(
            "extensions/remove",
            serde_json::json!({
                "extension": extension,
            }),
        )
        .await
    }

    /// Update an extension (git pull + rebuild). Pass None to update all.
    pub async fn extensions_update(&self, extension: Option<&str>) -> Result<serde_json::Value> {
        let mut params = serde_json::json!({});
        if let Some(name) = extension {
            params["extension"] = serde_json::Value::String(name.into());
        }
        self.ext_call("extensions/update", params).await
    }

    /// List available skills (bundled + project-local).
    pub async fn skills_list(&self) -> Result<serde_json::Value> {
        self.ext_call("skills/list", serde_json::json!({})).await
    }

    /// Install all bundled skills to ~/.omegon/skills/.
    pub async fn skills_install(&self) -> Result<serde_json::Value> {
        self.ext_call("skills/install", serde_json::json!({})).await
    }

    // ── Control requests (TUI parity) ──────────────────────────

    /// Generic control request — maps to TUI slash commands.
    async fn control_call(&self, command: &str, args: &str) -> Result<serde_json::Value> {
        let mut params = serde_json::json!({});
        if !args.is_empty() {
            params["args"] = serde_json::Value::String(args.into());
        }
        self.ext_call(&format!("control/{command}"), params).await
    }

    /// Session statistics (model, turns, context usage, etc.)
    pub async fn stats(&self) -> Result<serde_json::Value> {
        self.control_call("stats", "").await
    }

    /// Get or set max turns.
    pub async fn max_turns(&self, value: Option<u32>) -> Result<serde_json::Value> {
        let args = value.map(|v| v.to_string()).unwrap_or_default();
        self.control_call("max_turns", &args).await
    }

    /// List available personas.
    pub async fn persona_list(&self) -> Result<serde_json::Value> {
        self.control_call("persona_list", "").await
    }

    /// Switch persona.
    pub async fn persona_switch(&self, name: &str) -> Result<serde_json::Value> {
        self.control_call("persona_switch", name).await
    }

    /// View current profile (model, thinking, posture, context window).
    pub async fn profile_view(&self) -> Result<serde_json::Value> {
        self.control_call("profile_view", "").await
    }

    /// Context usage status.
    pub async fn context_status(&self) -> Result<serde_json::Value> {
        self.control_call("context_status", "").await
    }

    /// Get or set context class.
    pub async fn context_class(&self, class: Option<&str>) -> Result<serde_json::Value> {
        self.control_call("context_class", class.unwrap_or(""))
            .await
    }

    /// Get or set runtime mode (slim/standard).
    pub async fn runtime_mode(&self, mode: Option<&str>) -> Result<serde_json::Value> {
        self.control_call("runtime_mode", mode.unwrap_or("")).await
    }

    /// View configured secrets (names + recipes, no values).
    pub async fn secrets_view(&self) -> Result<serde_json::Value> {
        self.control_call("secrets_view", "").await
    }

    /// Project status.
    pub async fn project_status(&self) -> Result<serde_json::Value> {
        self.control_call("project_status", "").await
    }

    /// Auth status.
    pub async fn auth_status(&self) -> Result<serde_json::Value> {
        self.control_call("auth_status", "").await
    }

    /// Provider status — per-provider auth state (authenticated/expired/missing).
    pub async fn provider_status(&self) -> Result<serde_json::Value> {
        self.control_call("provider_status", "").await
    }

    /// Add a session note.
    pub async fn note_add(&self, text: &str) -> Result<serde_json::Value> {
        self.control_call("note_add", text).await
    }

    /// View all notes.
    pub async fn notes_view(&self) -> Result<serde_json::Value> {
        self.control_call("notes_view", "").await
    }

    /// Clear all notes.
    pub async fn notes_clear(&self) -> Result<serde_json::Value> {
        self.control_call("notes_clear", "").await
    }

    /// Workspace status.
    pub async fn workspace_status(&self) -> Result<serde_json::Value> {
        self.control_call("workspace_status", "").await
    }

    /// List all workspaces.
    pub async fn workspace_list(&self) -> Result<serde_json::Value> {
        self.control_call("workspace_list", "").await
    }

    /// Design tree view.
    pub async fn tree_view(&self, args: &str) -> Result<serde_json::Value> {
        self.control_call("tree_view", args).await
    }

    // ── Discovery ──────────────────────────────────────────────

    /// Browse the Armory for available extensions using Omegon's current ACP standard.
    pub async fn armory_search_extensions(&self, query: Option<&str>) -> Result<serde_json::Value> {
        let mut params = serde_json::json!({ "kind": "extensions" });
        if let Some(q) = query {
            params["query"] = serde_json::Value::String(q.into());
        }
        self.ext_call("armory/search", params).await
    }

    /// Install an item from the Armory registry.
    pub async fn armory_install_extension(&self, target: &str) -> Result<serde_json::Value> {
        self.ext_call(
            "armory/install",
            serde_json::json!({
                "target": target,
                "kind": "extensions",
            }),
        )
        .await
    }

    /// Install a package through Omegon's package substrate. Supports git URLs,
    /// local paths, Armory refs, extension manifests, and plugin.toml skill packages.
    pub async fn packages_install(
        &self,
        source: &str,
        kind_hint: &str,
    ) -> Result<serde_json::Value> {
        self.ext_call(
            "packages/install",
            serde_json::json!({
                "source": source,
                "kind_hint": kind_hint,
            }),
        )
        .await
    }

    /// Actively probe the Flynt extension deployment metadata after ACP connects.
    ///
    /// ACP readiness only proves transport/session health. Flynt still needs to
    /// verify that Omegon's project extension is scoped to the same vault and is
    /// speaking the expected Flynt contract.
    pub async fn flynt_deployment_probe(&self) -> Result<serde_json::Value> {
        let extensions = self.extensions_list().await?;
        let flynt = extensions
            .get("extensions")
            .and_then(|value| value.as_array())
            .and_then(|extensions| {
                extensions.iter().find(|extension| {
                    extension
                        .get("id")
                        .or_else(|| extension.get("name"))
                        .and_then(|value| value.as_str())
                        == Some("flynt")
                })
            });

        let Some(flynt) = flynt else {
            return Ok(serde_json::json!({
                "flynt_probe": {
                    "status": "missing",
                    "message": "flynt extension is not installed"
                }
            }));
        };

        let enabled = flynt
            .get("enabled")
            .and_then(|value| value.as_bool())
            .unwrap_or(true);
        let loaded = flynt
            .get("loaded")
            .and_then(|value| value.as_bool())
            .unwrap_or(enabled);
        let callable = flynt
            .get("callable")
            .and_then(|value| value.as_bool())
            .unwrap_or(loaded);
        if !enabled {
            return Ok(serde_json::json!({
                "flynt_probe": {
                    "status": "disabled",
                    "message": "flynt extension is installed but disabled",
                    "enabled": enabled,
                    "loaded": loaded,
                    "callable": callable,
                    "last_error": flynt.get("last_error").cloned().unwrap_or(serde_json::Value::Null),
                    "extension": flynt,
                }
            }));
        }

        let call_result = self
            .ext_call(
                "extensions/call",
                serde_json::json!({
                    "extension": "flynt",
                    "method": "initialize",
                    "params": {},
                }),
            )
            .await;

        match call_result {
            Ok(value) => Ok(value.get("result").cloned().unwrap_or(value)),
            Err(error) => Ok(serde_json::json!({
                "flynt_probe": {
                    "status": if !loaded { "not_loaded" } else if !callable { "not_callable" } else { "call_failed" },
                    "message": error.to_string(),
                    "enabled": enabled,
                    "loaded": loaded,
                    "callable": callable,
                    "last_error": flynt.get("last_error").cloned().unwrap_or(serde_json::Value::Null),
                    "extension": flynt,
                }
            })),
        }
    }

    /// Search the armory registry for available extensions.
    pub async fn extensions_search(&self, query: Option<&str>) -> Result<serde_json::Value> {
        let mut params = serde_json::json!({});
        if let Some(q) = query {
            params["query"] = serde_json::Value::String(q.into());
        }
        self.ext_call("extensions/search", params).await
    }

    /// List agent catalog (bundled + installed agents).
    pub async fn catalog_list(&self) -> Result<serde_json::Value> {
        self.ext_call("catalog/list", serde_json::json!({})).await
    }

    // ── Omegon plan/task projections (read-only) ─────────────────────

    /// Query Omegon runtime capabilities. This is read-only discovery.
    pub async fn omegon_runtime_capabilities(&self) -> Result<OmegonRuntimeCapabilities> {
        let value = self.ext_call("_runtime/capabilities", serde_json::json!({})).await?;
        parse_projection(value, "capabilities")
    }

    /// List projected Omegon plans as read-only external planning context.
    pub async fn omegon_plans_list(&self) -> Result<Vec<OmegonPlanSummary>> {
        let value = self.ext_call("_plans/list", serde_json::json!({})).await?;
        parse_projection(value, "plans")
    }

    /// Show one projected Omegon plan. Does not mutate or bind anything.
    pub async fn omegon_plan_show(&self, plan_id: &str) -> Result<OmegonPlanDetail> {
        let value = self
            .ext_call("_plans/show", serde_json::json!({ "plan_id": plan_id }))
            .await?;
        parse_projection(value, "plan")
    }

    /// List projected Omegon tasks as read-only external planning context.
    pub async fn omegon_tasks_list(&self, plan_id: Option<&str>) -> Result<Vec<OmegonTaskSummary>> {
        let mut params = serde_json::json!({});
        if let Some(plan_id) = plan_id {
            params["plan_id"] = serde_json::Value::String(plan_id.into());
        }
        let value = self.ext_call("_tasks/list", params).await?;
        parse_projection(value, "tasks")
    }

    /// Show one projected Omegon task. Does not call `_tasks/bind`.
    pub async fn omegon_task_show(&self, task_id: &str) -> Result<OmegonTaskDetail> {
        let value = self
            .ext_call("_tasks/show", serde_json::json!({ "task_id": task_id }))
            .await?;
        parse_projection(value, "task")
    }

    /// Install agents from the armory (or bundled fallback).
    pub async fn catalog_install(&self, offline: bool) -> Result<serde_json::Value> {
        self.ext_call("catalog/install", serde_json::json!({ "offline": offline }))
            .await
    }

    // ── Persona CRUD ───────────────────────────────────────────

    /// Create a new persona.
    pub async fn persona_create(
        &self,
        name: &str,
        directive: &str,
        description: &str,
        badge: Option<&str>,
        disabled_tools: &[String],
    ) -> Result<serde_json::Value> {
        let mut params = serde_json::json!({
            "name": name,
            "directive": directive,
            "description": description,
            "disabled_tools": disabled_tools,
        });
        if let Some(b) = badge {
            params["badge"] = serde_json::Value::String(b.into());
        }
        self.ext_call("personas/create", params).await
    }

    /// Delete a persona by ID.
    pub async fn persona_delete(&self, id: &str) -> Result<serde_json::Value> {
        self.ext_call("personas/delete", serde_json::json!({ "id": id }))
            .await
    }

    // ── Skill CRUD ─────────────────────────────────────────────

    /// Create a custom skill.
    pub async fn skill_create(&self, name: &str, content: &str) -> Result<serde_json::Value> {
        self.ext_call(
            "skills/create",
            serde_json::json!({
                "name": name,
                "content": content,
            }),
        )
        .await
    }

    /// Delete a skill by name.
    pub async fn skill_delete(&self, name: &str) -> Result<serde_json::Value> {
        self.ext_call(
            "skills/delete",
            serde_json::json!({
                "name": name,
            }),
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn choose_allow_prefers_allow_once() {
        let options = vec![
            PermissionOption::new("always", "Allow always", PermissionOptionKind::AllowAlways),
            PermissionOption::new("once", "Allow once", PermissionOptionKind::AllowOnce),
        ];
        assert_eq!(
            choose_option(
                &options,
                &[
                    PermissionOptionKind::AllowOnce,
                    PermissionOptionKind::AllowAlways
                ]
            )
            .unwrap()
            .to_string(),
            "once"
        );
    }

    #[test]
    fn choose_reject_prefers_reject_once() {
        let options = vec![
            PermissionOption::new(
                "always",
                "Reject always",
                PermissionOptionKind::RejectAlways,
            ),
            PermissionOption::new("once", "Reject once", PermissionOptionKind::RejectOnce),
        ];
        assert_eq!(
            choose_option(
                &options,
                &[
                    PermissionOptionKind::RejectOnce,
                    PermissionOptionKind::RejectAlways
                ]
            )
            .unwrap()
            .to_string(),
            "once"
        );
    }

    #[test]
    fn unwrap_projection_payload_accepts_direct_key() {
        let value = serde_json::json!({
            "plans": [{ "id": "plan-a", "title": "Plan A" }],
            "other": true,
        });

        let plans: Vec<OmegonPlanSummary> = parse_projection(value, "plans").unwrap();

        assert_eq!(plans.len(), 1);
        assert_eq!(plans[0].id, "plan-a");
        assert_eq!(plans[0].title.as_deref(), Some("Plan A"));
    }

    #[test]
    fn unwrap_projection_payload_accepts_result_wrapped_key() {
        let value = serde_json::json!({
            "result": {
                "tasks": [{
                    "id": "task-a",
                    "stable_id": "stable-task-a",
                    "source": { "kind": "openspec", "path": "openspec/changes/foo/tasks.md", "anchor": "1.1" },
                    "plan_id": "plan-a",
                    "label": "Task A",
                    "status": "pending",
                    "revision": "source-v1:openspec:foo:1.1:sha256:abc",
                    "writable": true,
                    "supported_mutations": ["bind_external_ref"]
                }]
            }
        });

        let tasks: Vec<OmegonTaskSummary> = parse_projection(value, "tasks").unwrap();

        assert_eq!(tasks.len(), 1);
        assert_eq!(tasks[0].id, "task-a");
        assert_eq!(tasks[0].stable_id.as_deref(), Some("stable-task-a"));
        assert_eq!(tasks[0].source.as_ref().map(|source| source.kind.as_str()), Some("openspec"));
        assert_eq!(tasks[0].source.as_ref().and_then(|source| source.anchor.as_deref()), Some("1.1"));
        assert_eq!(tasks[0].supported_mutations, vec!["bind_external_ref"]);
    }

    #[test]
    fn task_detail_preserves_typed_source_and_unknown_projection_fields() {
        let value = serde_json::json!({
            "task": {
                "id": "task-a",
                "source": { "kind": "openspec", "path": "openspec/changes/foo/tasks.md" },
                "custom": { "nested": true }
            }
        });

        let task: OmegonTaskDetail = parse_projection(value, "task").unwrap();

        assert_eq!(task.id, "task-a");
        assert_eq!(task.source.as_ref().map(|source| source.kind.as_str()), Some("openspec"));
        assert_eq!(
            task.extra.get("custom").and_then(|custom| custom.get("nested")).and_then(|nested| nested.as_bool()),
            Some(true)
        );
    }

    #[test]
    fn runtime_capabilities_parse_plan_tasks_contract() {
        let value = serde_json::json!({
            "features": {
                "plan_tasks_contract": {
                    "compatibility": ["read_only", "manual_link", "session_bind"],
                    "stable_id": true,
                    "revision": true,
                    "durable_bind": false,
                    "structured_errors": true,
                    "pagination": false,
                    "filtering": true
                }
            }
        });

        let capabilities: OmegonRuntimeCapabilities = parse_projection(value, "capabilities").unwrap();
        let contract = capabilities.plan_tasks_contract().unwrap();

        assert_eq!(contract.compatibility, vec!["read_only", "manual_link", "session_bind"]);
        assert!(contract.stable_id);
        assert!(contract.revision);
        assert!(!contract.durable_bind);
        assert!(contract.structured_errors);
        assert!(contract.filtering);
        assert!(!contract.pagination);
    }
}
