use crate::bootstrap::AppContext;
use crate::state::TerminalOpenCommand;
use crate::terminal::{
    TerminalCreateParams, TerminalManager, TerminalPlacement, TerminalSessionInfo,
    TerminalSnapshotView, TerminalStatus,
};
use dioxus::prelude::*;

const TERMINAL_ROWS: usize = 34;
const TERMINAL_COLS: usize = 120;
#[component]
pub fn TerminalLabView() -> Element {
    let ctx = use_context::<AppContext>();
    let project_root = ctx.project_root();
    let manager = use_context::<TerminalManager>();
    let mut terminal_id = use_signal(|| None::<String>);
    let terminal_open = use_context::<Signal<TerminalOpenCommand>>();
    let mut last_open_version = use_signal(|| 0_u64);
    let mut sessions = use_signal(Vec::<TerminalSessionInfo>::new);
    let mut status = use_signal(|| TerminalStatus::Failed("not started".to_string()));
    let mut snapshot = use_signal(|| crate::terminal::view::TerminalSnapshot::blank(TERMINAL_ROWS, TERMINAL_COLS));
    let mut error = use_signal(|| None::<String>);

    {
        let manager = manager.clone();
        let project_root = project_root.clone();
        use_effect(move || {
            if !manager.list().is_empty() {
                sessions.set(manager.list());
                return;
            }
            let shell = std::env::var("SHELL").unwrap_or_else(|_| "sh".to_string());
            let mut params = TerminalCreateParams::new(shell);
            params.cwd = Some(project_root.display().to_string());
            params.title = Some("Shell".to_string());
            params.placement = Some(TerminalPlacement::BottomPane);
            params.reuse_key = Some("terminal-shell".to_string());
            match manager.create(params) {
                Ok(result) => {
                    terminal_id.set(Some(result.terminal_id));
                    sessions.set(manager.list());
                }
                Err(err) => {
                    error.set(Some(err.to_string()));
                }
            }
        });
    }


    {
        let command = terminal_open.read().clone();
        if command.version != *last_open_version.read() {
            last_open_version.set(command.version);
            if let Some(id) = command.terminal_id {
                terminal_id.set(Some(id));
            }
        }
    }

    {
        let manager = manager.clone();
        use_future(move || {
            let manager = manager.clone();
            async move {
            loop {
                if let Some(id) = terminal_id.read().clone() {
                    if let Ok(next) = manager.poll_snapshot(&id) {
                        snapshot.set(next);
                    }
                    if let Ok(next_status) = manager.status(&id) {
                        status.set(next_status);
                    }
                }
                sessions.set(manager.list());
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
            }
        });
    }

    let status_label = match &*status.read() {
        TerminalStatus::Running => "running".to_string(),
        TerminalStatus::Exited(label) => format!("exited ({label})"),
        TerminalStatus::Failed(err) => format!("failed ({err})"),
    };
    let manager_for_kill = manager.clone();
    let manager_for_release = manager.clone();
    let manager_for_release_exited = manager.clone();
    let manager_for_input = manager.clone();
    let manager_for_resize = manager.clone();

    rsx! {
        div { class: "terminal-view",
            div { class: "terminal-header",
                div {
                    h2 { "Terminal" }
                    p { "Flynt terminal dogfoods the reusable TerminalManager lifecycle: create, snapshot, status, input, kill, release." }
                    div { class: "terminal-engine", "Engine: portable-pty + alacritty_terminal + Flynt renderer" }
                }
                div { class: "terminal-meta",
                    div { "Project: {project_root.display()}" }
                    div { "Status: {status_label}" }
                }
            }
            div { class: "terminal-checks",
                span { "Checks:" }
                span { "ANSI/truecolor" }
                span { "Unicode width" }
                span { "scrollback" }
                span { "keyboard input" }
                span { "resize" }
                span { "interactive shell" }
                button {
                    class: "terminal-action-btn",
                    onclick: move |_| {
                        let _ = manager_for_release_exited.release_all_exited();
                        terminal_id.set(None);
                    },
                    "Clear exited"
                }
                if let Some(id) = terminal_id.read().clone() {
                    {
                        let kill_id = id.clone();
                        let release_id = id.clone();
                        rsx! {
                            button {
                                class: "terminal-action-btn",
                                onclick: move |_| {
                                    let _ = manager_for_kill.kill(&kill_id);
                                },
                                "Kill"
                            }
                            button {
                                class: "terminal-action-btn",
                                onclick: move |_| {
                                    let _ = manager_for_release.release(&release_id);
                                    terminal_id.set(None);
                                },
                                "Release"
                            }
                        }
                    }
                }
            }

            div { class: "terminal-session-strip",
                span { class: "terminal-session-label", "Sessions:" }
                for session in sessions.read().iter() {
                    {
                        let id = session.terminal_id.clone();
                        let selected = terminal_id.read().as_deref() == Some(id.as_str());
                        let status_text = terminal_status_label(&session.status);
                        rsx! {
                            button {
                                key: "term-session-{id}",
                                class: if selected { "terminal-session-btn active" } else { "terminal-session-btn" },
                                onclick: move |_| terminal_id.set(Some(id.clone())),
                                span { class: "terminal-session-title", "{session.title}" }
                                span { class: "terminal-session-command", "{session.command_line}" }
                                span { class: "terminal-session-status", "{status_text}" }
                            }
                        }
                    }
                }
            }
            if let Some(err) = error.read().clone() {
                div { class: "terminal-error", "Terminal error: {err}" }
            }
            div { class: "terminal-frame",
                TerminalSnapshotView {
                    snapshot: snapshot.read().clone(),
                    font_size: 13,
                    class: "flynt-terminal".to_string(),
                    on_key: move |input: String| {
                        if let Some(id) = terminal_id.read().clone() {
                            let _ = manager_for_input.send_input(&id, &input);
                        }
                    },
                    on_size: move |(rows, cols): (usize, usize)| {
                        if let Some(id) = terminal_id.read().clone() {
                            let _ = manager_for_resize.resize(&id, rows, cols);
                        }
                    },
                }
            }
        }
    }
}

fn terminal_status_label(status: &TerminalStatus) -> String {
    match status {
        TerminalStatus::Running => "running".to_string(),
        TerminalStatus::Exited(label) => format!("exited {label}"),
        TerminalStatus::Failed(err) => format!("failed {err}"),
    }
}
