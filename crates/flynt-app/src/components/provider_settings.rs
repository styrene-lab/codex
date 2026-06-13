use crate::{acp::AcpSession, bootstrap::AppContext};
use dioxus::prelude::*;
use flynt_core::providers::{self, AuthMethod, CredentialStatus, ProviderInfo};
use std::{collections::HashMap, rc::Rc};

#[derive(Clone, PartialEq)]
enum ProviderActionMessage {
    Info(String),
    Progress(String),
    Success(String),
    Error(String),
}

impl ProviderActionMessage {
    fn text(&self) -> &str {
        match self {
            Self::Info(text) | Self::Progress(text) | Self::Success(text) | Self::Error(text) => text,
        }
    }

    fn should_render(&self, is_authenticated: bool, operation_active: bool) -> bool {
        match self {
            Self::Success(_) => is_authenticated,
            Self::Progress(_) => operation_active,
            Self::Info(_) | Self::Error(_) => true,
        }
    }
}

fn parse_live_provider_status(text: &str) -> HashMap<String, CredentialStatus> {
    text.lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.splitn(3, ':').collect();
            if parts.len() < 2 {
                return None;
            }
            let id = parts[0].trim().to_string();
            let status = match parts[1].trim() {
                "ok" | "authenticated" | "available" | "configured" => {
                    CredentialStatus::Authenticated {
                        source: parts
                            .get(2)
                            .map(|s| s.trim())
                            .filter(|s| !s.is_empty())
                            .unwrap_or("omegon")
                            .to_string(),
                    }
                }
                "expired" => CredentialStatus::Expired,
                _ => CredentialStatus::Missing,
            };
            Some((id, status))
        })
        .collect()
}

async fn live_provider_status_for(
    sess: Rc<AcpSession>,
    provider_id: &str,
) -> Option<CredentialStatus> {
    let resp = sess.provider_status().await.ok()?;
    let text = resp["text"].as_str()?;
    parse_live_provider_status(text).remove(provider_id)
}

async fn wait_for_authenticated_provider(sess: Rc<AcpSession>, provider_id: &str) -> bool {
    for _ in 0..10 {
        if matches!(
            live_provider_status_for(sess.clone(), provider_id).await,
            Some(CredentialStatus::Authenticated { .. })
        ) {
            return true;
        }
        tokio::time::sleep(std::time::Duration::from_secs(2)).await;
    }
    false
}

#[component]
pub fn ProviderSettingsSection() -> Element {
    let shared_session = use_context::<Signal<Option<Rc<AcpSession>>>>();
    let mut refresh = use_signal(|| 0u64);

    let statuses = use_resource(move || {
        let _ = refresh.read();
        let sess = shared_session.read().clone();
        async move {
            let mut local = tokio::task::spawn_blocking(providers::probe_all)
                .await
                .unwrap_or_default();
            if let Some(sess) = sess {
                if let Ok(resp) = sess.provider_status().await {
                    if let Some(text) = resp["text"].as_str() {
                        let live = parse_live_provider_status(text);
                        for (provider, status) in local.iter_mut() {
                            if let Some(live_status) = live.get(provider.id) {
                                *status = live_status.clone();
                            }
                        }
                    }
                }
            }
            local
        }
    });

    rsx! {
        section { class: "settings-section",
            h2 { class: "settings-heading", "Providers" }
            div { class: "settings-rows",
                for (provider, status) in statuses.read().as_ref().unwrap_or(&vec![]).iter() {
                    ProviderRow {
                        provider,
                        status: status.clone(),
                        session: shared_session,
                        on_change: move |_| *refresh.write() += 1,
                    }
                }
            }
        }
    }
}

#[component]
fn ProviderRow(
    provider: &'static ProviderInfo,
    status: CredentialStatus,
    session: Signal<Option<Rc<AcpSession>>>,
    on_change: EventHandler<()>,
) -> Element {
    let ctx = use_context::<AppContext>();
    let mut editing = use_signal(|| false);
    let mut key_input = use_signal(String::new);
    let mut error_msg: Signal<Option<String>> = use_signal(|| None);
    let mut action_msg: Signal<Option<ProviderActionMessage>> = use_signal(|| None);
    let mut logging_in = use_signal(|| false);

    let (status_class, status_text) = match &status {
        CredentialStatus::Authenticated { source } => (
            "provider-status authenticated",
            format!("Authenticated ({source})"),
        ),
        CredentialStatus::Expired => ("provider-status expired", "Expired".to_string()),
        CredentialStatus::Missing => ("provider-status missing", "Not configured".to_string()),
    };

    let is_authenticated = matches!(status, CredentialStatus::Authenticated { .. });
    let is_api_key = provider.auth_method == AuthMethod::ApiKey;

    let operation_active = *logging_in.read() || *editing.read();
    let rendered_action_msg = action_msg
        .read()
        .as_ref()
        .filter(|msg| msg.should_render(is_authenticated, operation_active))
        .map(|msg| msg.text().to_string());

    rsx! {
        div { class: "settings-row provider-row",
            span { class: "settings-label", "{provider.label}" }
            div { class: "settings-control",
                div { class: "provider-status-row",
                    span { class: status_class }
                    span { class: "provider-status-text", "{status_text}" }
                }

                if *editing.read() {
                    div { class: "provider-key-form",
                        input {
                            class: "input settings-input",
                            r#type: "password",
                            value: "{key_input}",
                            placeholder: if is_api_key { "API key…" } else { "OAuth token…" },
                            autofocus: true,
                            oninput: move |e| *key_input.write() = e.value(),
                            onkeydown: move |e| {
                                if e.key() == Key::Escape {
                                    *editing.write() = false;
                                    *key_input.write() = String::new();
                                }
                            },
                        }
                        div { class: "row gap-2",
                            button {
                                class: "btn btn-primary btn-sm",
                                disabled: key_input.read().trim().is_empty(),
                                onclick: move |_| {
                                    let key = key_input.read().trim().to_string();
                                    if key.is_empty() { return; }
                                    match providers::save_api_key(provider.id, &key) {
                                        Ok(()) => {
                                            *editing.write() = false;
                                            *key_input.write() = String::new();
                                            *error_msg.write() = None;
                                            *action_msg.write() = Some(ProviderActionMessage::Info("Credential saved. Refreshing provider status…".into()));
                                            on_change.call(());
                                        }
                                        Err(e) => *error_msg.write() = Some(format!("{e}")),
                                    }
                                },
                                "Save"
                            }
                            button {
                                class: "btn btn-ghost btn-sm",
                                onclick: move |_| {
                                    *editing.write() = false;
                                    *key_input.write() = String::new();
                                },
                                "Cancel"
                            }
                        }
                        if let Some(ref err) = *error_msg.read() {
                            span { class: "text-error", "{err}" }
                        }
                    }
                } else {
                    div { class: "row gap-2",
                        if is_api_key {
                            button {
                                class: "btn btn-ghost btn-sm",
                                onclick: move |_| *editing.write() = true,
                                if is_authenticated { "Update key" } else { "Add key" }
                            }
                        } else {
                            button {
                                class: "btn btn-ghost btn-sm",
                                disabled: *logging_in.read(),
                                onclick: move |_| {
                                    let Some(sess) = session.read().clone() else {
                                        *action_msg.write() = Some(ProviderActionMessage::Error("Login requires a connected Omegon session.".into()));
                                        return;
                                    };
                                    let binary = ctx.omegon().resolve_binary();
                                    let provider_id = provider.id.to_string();
                                    *logging_in.write() = true;
                                    *action_msg.write() = Some(ProviderActionMessage::Progress(format!("Opening {} login…", provider.label)));
                                    spawn(async move {
                                        match sess.login(&binary, &provider_id).await {
                                            Ok(_) => {
                                                *action_msg.write() = Some(ProviderActionMessage::Progress(format!("{} login returned successfully. Waiting for provider status…", provider.label)));
                                                on_change.call(());
                                                if wait_for_authenticated_provider(sess.clone(), &provider_id).await {
                                                    *action_msg.write() = Some(ProviderActionMessage::Success(format!("{} authenticated.", provider.label)));
                                                } else {
                                                    *action_msg.write() = Some(ProviderActionMessage::Info(format!("{} login finished, but live provider status is not authenticated yet. Reopen this panel or reconnect Omegon if it remains stale.", provider.label)));
                                                }
                                                on_change.call(());
                                            }
                                            Err(error) => {
                                                *action_msg.write() = Some(ProviderActionMessage::Error(format!("{} login failed: {error}", provider.label)));
                                            }
                                        }
                                        *logging_in.write() = false;
                                    });
                                },
                                if *logging_in.read() { "Logging in…" } else if is_authenticated { "Re-authenticate" } else { "Login" }
                            }
                            button {
                                class: "btn btn-ghost btn-sm",
                                onclick: move |_| *editing.write() = true,
                                "Paste token"
                            }
                        }

                        if is_authenticated {
                            button {
                                class: "btn btn-ghost btn-sm provider-remove-btn",
                                onclick: move |_| {
                                    let _ = providers::remove_credential(provider.id);
                                    *action_msg.write() = Some(ProviderActionMessage::Info("Credential removed. Refreshing provider status…".into()));
                                    on_change.call(());
                                },
                                "Remove"
                            }
                        }
                    }
                }
                if let Some(msg) = rendered_action_msg {
                    span { class: "settings-hint muted", "{msg}" }
                }
            }
        }
    }
}
