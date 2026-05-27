use crate::bootstrap::AppContext;
use crate::state::TerminalOpenCommand;
use crate::terminal::{
    TerminalCreateParams, TerminalManager, TerminalPlacement, TerminalSessionInfo,
    TerminalSnapshotView, TerminalStatus,
};
use dioxus::prelude::*;
use std::path::PathBuf;

const TERMINAL_ROWS: usize = 34;
const TERMINAL_COLS: usize = 120;
const TERMINAL_ID: &str = "terminal-diagnostics";

const TERMINAL_DIAGNOSTIC_SCRIPT: &str = r#"printf '\033[1;36mFlynt terminal\033[0m\n'
printf 'cwd: %s\n' "$PWD"
printf 'shell: %s\n' "${SHELL:-unknown}"
printf 'term: %s\n' "${TERM:-unset}"
printf '\n\033[1mRequired behavior checklist\033[0m\n'
printf '[1] ANSI SGR colors: \033[31mred\033[0m \033[32mgreen\033[0m \033[34mblue\033[0m \033[38;2;42;180;200mtruecolor\033[0m\n'
printf '[2] Unicode width: ASCII | λ hydra 🜁 | box ┌─┐ │ │ └─┘\n'
printf '[3] Nerd/powerline glyphs: branch  lock  prompt ❯ separator \n'
printf '[4] Width stress: emoji 😀 ⚙️ 🧪 | CJK 界面 終端 | combining e\u0301 a\u0308 o\u0302\n'
printf '[5] OSC/title safety: setting terminal title should not leak raw escapes\n'
printf '\033]0;Flynt terminal title\007'
printf '[6] Cursor/control: carriage return overwrite -> start'; printf '\r[6] Cursor/control: carriage return overwrite -> ok   \n'
printf '[7] Long output / scrollback follows:\n'
for i in $(seq 1 40); do printf '    scrollback line %02d: terminal output remains readable\n' "$i"; done
printf '[8] Resize requirement: grid must update rows/cols without corrupting output\n'
printf '[9] Input requirement: after this script exits, an interactive shell should accept typed commands\n'
printf '[10] Process lifecycle: Flynt must detect exit status and clean up PTY/threads\n'
printf '\n\033[1;33mManual checks:\033[0m paste text, run less/vim/top if available, resize window, then exit.\n'
printf '\nLaunching interactive shell...\n'
exec "${SHELL:-/bin/sh}" -l
"#;

#[component]
pub fn TerminalLabView() -> Element {
    let ctx = use_context::<AppContext>();
    let project_root = ctx.project_root();
    let manager = use_context::<TerminalManager>();
    let script_path = ensure_diagnostic_script(&project_root);
    let script_display = script_path.display().to_string();
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
        let script_arg = script_path.to_string_lossy().to_string();
        use_effect(move || {
            let mut params = TerminalCreateParams::new("sh");
            params.args = vec![script_arg.clone()];
            params.cwd = Some(project_root.display().to_string());
            params.title = Some("Flynt Terminal".to_string());
            params.placement = Some(TerminalPlacement::BottomPane);
            params.reuse_key = Some(TERMINAL_ID.to_string());
            match manager.create(params) {
                Ok(result) => {
                    terminal_id.set(Some(result.terminal_id));
                    sessions.set(manager.list());
                    error.set(None);
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
                    div { "Script: {script_display}" }
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

fn ensure_diagnostic_script(project_root: &PathBuf) -> PathBuf {
    let dir = project_root.join(".flynt-local").join("terminal");
    let path = dir.join("terminal-necessities.sh");
    if let Err(err) = std::fs::create_dir_all(&dir) {
        tracing::warn!("Failed to create terminal dir {}: {err}", dir.display());
        return path;
    }
    if let Err(err) = std::fs::write(&path, TERMINAL_DIAGNOSTIC_SCRIPT) {
        tracing::warn!("Failed to write terminal script {}: {err}", path.display());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Ok(meta) = std::fs::metadata(&path) {
            let mut perms = meta.permissions();
            perms.set_mode(0o755);
            let _ = std::fs::set_permissions(&path, perms);
        }
    }
    path
}
