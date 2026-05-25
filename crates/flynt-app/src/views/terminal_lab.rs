use crate::bootstrap::AppContext;
use crate::terminal::AlacrittyTerminal;
use dioxus::prelude::*;
use std::path::PathBuf;

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
    let script_path = ensure_diagnostic_script(&project_root);
    let script_display = script_path.display().to_string();

    rsx! {
        div { class: "terminal-view",
            div { class: "terminal-header",
                div {
                    h2 { "Terminal" }
                    p { "MVP Flynt terminal using portable-pty + alacritty_terminal. This is a terminal surface, not the final HostAction terminal UX." }
                    div { class: "terminal-engine", "Engine: portable-pty + alacritty_terminal + Flynt renderer" }
                }
                div { class: "terminal-meta",
                    div { "Project: {project_root.display()}" }
                    div { "Script: {script_display}" }
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
            }
            div { class: "terminal-frame",
                AlacrittyTerminal {
                    command: "/bin/sh".to_string(),
                    args: vec![script_path.to_string_lossy().to_string()],
                    rows: 34,
                    cols: 120,
                    font_size: 13,
                    class: "flynt-terminal".to_string(),
                }
            }
        }
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
