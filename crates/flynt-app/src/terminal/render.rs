//! Dioxus rendering helpers for terminal snapshots.

use dioxus::html::geometry::WheelDelta;
use dioxus::prelude::*;

use super::view::{DEFAULT_COLS, DEFAULT_ROWS, TerminalSnapshot, key_to_terminal_input};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct TermColor(pub(crate) u8, pub(crate) u8, pub(crate) u8);

impl TermColor {
    pub(crate) const BG: Self = Self(6, 8, 14);
    pub(crate) const FG: Self = Self(196, 216, 228);
    pub(crate) const MUTED: Self = Self(96, 120, 136);

    pub(crate) fn css(self) -> String {
        format!("rgb({}, {}, {})", self.0, self.1, self.2)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RenderCell {
    pub(crate) text: String,
    pub(crate) fg: TermColor,
    pub(crate) bg: TermColor,
    pub(crate) bold: bool,
    pub(crate) italic: bool,
    pub(crate) underline: bool,
    pub(crate) inverse: bool,
    pub(crate) wide_spacer: bool,
}

impl Default for RenderCell {
    fn default() -> Self {
        Self {
            text: " ".to_string(),
            fg: TermColor::FG,
            bg: TermColor::BG,
            bold: false,
            italic: false,
            underline: false,
            inverse: false,
            wide_spacer: false,
        }
    }
}

#[component]
pub fn TerminalSnapshotView(props: TerminalSnapshotViewProps) -> Element {
    let font_family = "JetBrainsMono Nerd Font, JetBrains Mono, FiraCode Nerd Font, Fira Code, MesloLGS NF, Symbols Nerd Font Mono, Symbols Nerd Font, SF Mono, Menlo, Monaco, Cascadia Code, Consolas, ui-monospace, monospace";
    let on_key = props.on_key.clone();
    let on_paste = props.on_paste.clone();
    let on_scroll = props.on_scroll.clone();
    let on_size = props.on_size.clone();
    let visible_text = snapshot_visible_text(&props.snapshot);
    let visible_text_for_copy = visible_text.clone();
    let visible_text_for_key = visible_text.clone();

    rsx! {
        div {
            class: "flynt-alacritty-terminal {props.class}",
            tabindex: "0",
            onmounted: move |evt| {
                let on_size = on_size.clone();
                spawn(async move {
                    if let Ok(rect) = evt.get_client_rect().await {
                        let cols = (rect.size.width / (props.font_size as f64 * 0.60)).floor().max(20.0) as usize;
                        let rows = (rect.size.height / (props.font_size as f64 * 1.20)).floor().max(5.0) as usize;
                        on_size.call((rows, cols));
                        let _ = evt.set_focus(true).await;
                    }
                });
            },
            onpaste: move |_| {
                let on_paste = on_paste.clone();
                spawn(async move {
                    if let Ok(text) = document::eval("navigator.clipboard.readText()")
                        .recv::<String>()
                        .await
                    {
                        if !text.is_empty() {
                            on_paste.call(text);
                        }
                    }
                });
            },
            oncopy: move |evt| {
                evt.prevent_default();
                evt.stop_propagation();
                let text = visible_text_for_copy.clone();
                spawn(async move {
                    let script = format!(
                        "navigator.clipboard.writeText({});",
                        serde_json::to_string(&text).unwrap_or_else(|_| "\"\"".to_string())
                    );
                    let _ = document::eval(&script).await;
                });
            },
            onwheel: move |evt| {
                evt.prevent_default();
                evt.stop_propagation();
                let lines = match evt.data().delta() {
                    WheelDelta::Pixels(delta) => (delta.y / 18.0).round() as i32,
                    WheelDelta::Lines(delta) => delta.y.round() as i32,
                    WheelDelta::Pages(delta) => (delta.y * props.snapshot.rows.len() as f64).round() as i32,
                };
                if lines != 0 {
                    on_scroll.call(lines);
                }
            },
            onkeydown: move |evt| {
                evt.prevent_default();
                evt.stop_propagation();
                if let dioxus::prelude::Key::Character(s) = evt.key() {
                    let modifiers = evt.modifiers();
                    if (modifiers.meta() || modifiers.ctrl()) && s.eq_ignore_ascii_case("c") {
                        let text = visible_text_for_key.clone();
                        spawn(async move {
                            let script = format!(
                                "navigator.clipboard.writeText({});",
                                serde_json::to_string(&text)
                                    .unwrap_or_else(|_| "\"\"".to_string())
                            );
                            let _ = document::eval(&script).await;
                        });
                        return;
                    }
                    if (modifiers.meta() || modifiers.ctrl()) && s.eq_ignore_ascii_case("v") {
                        let on_paste = on_paste.clone();
                        spawn(async move {
                            if let Ok(text) = document::eval("navigator.clipboard.readText()")
                                .recv::<String>()
                                .await
                            {
                                if !text.is_empty() {
                                    on_paste.call(text);
                                }
                            }
                        });
                        return;
                    }
                }
                let input = key_to_terminal_input(&evt);
                if !input.is_empty() {
                    on_key.call(input);
                }
            },
            style: "background: {TermColor::BG.css()}; color: {TermColor::FG.css()}; font-family: {font_family}; font-size: {props.font_size}px; line-height: 1.2; overflow: hidden; white-space: pre; width: 100%; height: 100%; box-sizing: border-box; contain: paint;",
            for (row_idx, row) in props.snapshot.rows.iter().enumerate() {
                div { key: "row-{row_idx}", class: "flynt-terminal-row",
                    for (col_idx, cell) in row.iter().enumerate() {
                        {
                            let is_cursor = props.snapshot.cursor == (row_idx, col_idx);
                            let mut fg = cell.fg;
                            let mut bg = cell.bg;
                            if is_cursor {
                                std::mem::swap(&mut fg, &mut bg);
                            }
                            let decoration = if cell.underline { "text-decoration: underline;" } else { "" };
                            let weight = if cell.bold { "font-weight: 700;" } else { "" };
                            let style = if cell.italic { "font-style: italic;" } else { "" };
                            let display = if cell.wide_spacer { "" } else { cell.text.as_str() };
                            rsx! {
                                span {
                                    key: "cell-{row_idx}-{col_idx}",
                                    style: "color: {fg.css()}; background-color: {bg.css()}; {decoration} {weight} {style}",
                                    "{display}"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

fn snapshot_visible_text(snapshot: &TerminalSnapshot) -> String {
    snapshot
        .rows
        .iter()
        .map(|row| {
            row.iter()
                .filter(|cell| !cell.wide_spacer)
                .map(|cell| cell.text.as_str())
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Props, Clone, PartialEq)]
pub struct TerminalSnapshotViewProps {
    pub snapshot: TerminalSnapshot,
    #[props(default = 13)]
    pub font_size: u16,
    #[props(default)]
    pub class: String,
    pub on_key: EventHandler<String>,
    pub on_paste: EventHandler<String>,
    pub on_scroll: EventHandler<i32>,
    pub on_size: EventHandler<(usize, usize)>,
}

#[derive(Props, Clone, PartialEq)]
pub struct AlacrittyTerminalProps {
    pub command: String,
    #[props(default)]
    pub args: Vec<String>,
    #[props(default = DEFAULT_ROWS)]
    pub rows: usize,
    #[props(default = DEFAULT_COLS)]
    pub cols: usize,
    #[props(default = 13)]
    pub font_size: u16,
    #[props(default)]
    pub class: String,
}
