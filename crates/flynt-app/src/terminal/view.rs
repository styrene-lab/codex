//! Reusable terminal substrate spike using portable-pty and alacritty_terminal.
//!
//! This intentionally avoids `dioxus-terminal`: that crate proved useful for
//! PTY/layout exploration, but its renderer casts UTF-8 bytes directly to
//! chars. Keep this module free of Flynt-specific policy so it can be extracted
//! for Auspex or a shared Styrene terminal crate.

use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use alacritty_terminal::event::VoidListener;
use alacritty_terminal::index::Point;
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::test::TermSize;
use alacritty_terminal::term::Term;
use alacritty_terminal::vte::ansi::{Color as VteColor, NamedColor, Processor, Rgb};
use dioxus::prelude::*;
use portable_pty::{CommandBuilder, PtySize, native_pty_system};
use tokio::sync::mpsc;

const DEFAULT_ROWS: usize = 34;
const DEFAULT_COLS: usize = 120;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct TermColor(u8, u8, u8);

impl TermColor {
    const BG: Self = Self(6, 8, 14);
    const FG: Self = Self(196, 216, 228);
    const MUTED: Self = Self(96, 120, 136);

    fn css(self) -> String {
        format!("rgb({}, {}, {})", self.0, self.1, self.2)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RenderCell {
    text: String,
    fg: TermColor,
    bg: TermColor,
    bold: bool,
    italic: bool,
    underline: bool,
    inverse: bool,
    wide_spacer: bool,
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

#[derive(Clone, Debug, PartialEq, Eq)]
struct TerminalSnapshot {
    rows: Vec<Vec<RenderCell>>,
    cursor: (usize, usize),
}

impl TerminalSnapshot {
    fn blank(rows: usize, cols: usize) -> Self {
        Self {
            rows: vec![vec![RenderCell::default(); cols]; rows],
            cursor: (0, 0),
        }
    }
}

struct PtyHandle {
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    rx: mpsc::Receiver<Vec<u8>>,
}

pub struct AlacrittyTerminalSession {
    term: Term<VoidListener>,
    processor: Processor,
    pty: PtyHandle,
}

impl AlacrittyTerminalSession {
    pub fn spawn(command: &str, args: &[String], rows: usize, cols: usize) -> anyhow::Result<Self> {
        let size = PtySize {
            rows: rows as u16,
            cols: cols as u16,
            pixel_width: 0,
            pixel_height: 0,
        };
        let pair = native_pty_system().openpty(size)?;
        let mut cmd = CommandBuilder::new(command);
        cmd.args(args.iter().map(String::as_str));
        let _child = pair.slave.spawn_command(cmd)?;

        let writer = Arc::new(Mutex::new(pair.master.take_writer()?));
        let mut reader = pair.master.try_clone_reader()?;
        let (tx, rx) = mpsc::channel(512);
        thread::spawn(move || {
            let mut buf = [0_u8; 8192];
            loop {
                match reader.read(&mut buf) {
                    Ok(0) | Err(_) => break,
                    Ok(n) => {
                        if tx.blocking_send(buf[..n].to_vec()).is_err() {
                            break;
                        }
                    }
                }
            }
        });

        let size = TermSize::new(cols, rows);
        Ok(Self {
            term: Term::new(Default::default(), &size, VoidListener),
            processor: Processor::new(),
            pty: PtyHandle { writer, rx },
        })
    }

    fn poll(&mut self) -> bool {
        let mut changed = false;
        while let Ok(bytes) = self.pty.rx.try_recv() {
            self.processor.advance(&mut self.term, &bytes);
            changed = true;
        }
        changed
    }

    fn write_input(&self, input: &str) {
        if let Ok(mut writer) = self.pty.writer.lock() {
            let _ = writer.write_all(input.as_bytes());
            let _ = writer.flush();
        }
    }

    fn snapshot(&self, rows: usize, cols: usize) -> TerminalSnapshot {
        let content = self.term.renderable_content();
        let cursor = (
            (content.cursor.point.line.0.max(0) as usize).min(rows.saturating_sub(1)),
            content.cursor.point.column.0.min(cols.saturating_sub(1)),
        );
        let mut snapshot = TerminalSnapshot::blank(rows, cols);
        for indexed in content.display_iter {
            let Point { line, column } = indexed.point;
            let row = line.0;
            if row < 0 {
                continue;
            }
            let row = row as usize;
            let col = column.0;
            if row >= rows || col >= cols {
                continue;
            }
            let cell = indexed.cell;
            let flags = cell.flags;
            let mut text = cell.c.to_string();
            if let Some(zero_width) = cell.zerowidth() {
                for c in zero_width {
                    text.push(*c);
                }
            }
            let mut fg = resolve_color(cell.fg);
            let mut bg = resolve_color(cell.bg);
            let inverse = flags.contains(Flags::INVERSE);
            if inverse {
                std::mem::swap(&mut fg, &mut bg);
            }
            snapshot.rows[row][col] = RenderCell {
                text,
                fg,
                bg,
                bold: flags.contains(Flags::BOLD),
                italic: flags.contains(Flags::ITALIC),
                underline: flags.intersects(Flags::UNDERLINE | Flags::DOUBLE_UNDERLINE),
                inverse,
                wide_spacer: flags.contains(Flags::WIDE_CHAR_SPACER),
            };
        }
        snapshot.cursor = cursor;
        snapshot
    }
}

fn resolve_color(color: VteColor) -> TermColor {
    match color {
        VteColor::Spec(Rgb { r, g, b }) => TermColor(r, g, b),
        VteColor::Indexed(index) => indexed_color(index),
        VteColor::Named(NamedColor::Foreground) => TermColor::FG,
        VteColor::Named(NamedColor::Background) => TermColor::BG,
        VteColor::Named(NamedColor::Black) => TermColor(0, 0, 0),
        VteColor::Named(NamedColor::Red) => TermColor(200, 48, 48),
        VteColor::Named(NamedColor::Green) => TermColor(26, 184, 120),
        VteColor::Named(NamedColor::Yellow) => TermColor(184, 144, 32),
        VteColor::Named(NamedColor::Blue) => TermColor(42, 180, 200),
        VteColor::Named(NamedColor::Magenta) => TermColor(150, 120, 220),
        VteColor::Named(NamedColor::Cyan) => TermColor(110, 202, 216),
        VteColor::Named(NamedColor::White) => TermColor(196, 216, 228),
        VteColor::Named(NamedColor::BrightBlack) => TermColor::MUTED,
        VteColor::Named(NamedColor::BrightRed) => TermColor(255, 92, 92),
        VteColor::Named(NamedColor::BrightGreen) => TermColor(60, 220, 150),
        VteColor::Named(NamedColor::BrightYellow) => TermColor(220, 184, 72),
        VteColor::Named(NamedColor::BrightBlue) => TermColor(110, 202, 216),
        VteColor::Named(NamedColor::BrightMagenta) => TermColor(190, 160, 255),
        VteColor::Named(NamedColor::BrightCyan) => TermColor(140, 230, 240),
        VteColor::Named(NamedColor::BrightWhite) => TermColor(240, 248, 255),
        _ => TermColor::FG,
    }
}

fn indexed_color(index: u8) -> TermColor {
    const ANSI: [TermColor; 16] = [
        TermColor(0, 0, 0),
        TermColor(200, 48, 48),
        TermColor(26, 184, 120),
        TermColor(184, 144, 32),
        TermColor(42, 180, 200),
        TermColor(150, 120, 220),
        TermColor(110, 202, 216),
        TermColor(196, 216, 228),
        TermColor(96, 120, 136),
        TermColor(255, 92, 92),
        TermColor(60, 220, 150),
        TermColor(220, 184, 72),
        TermColor(110, 202, 216),
        TermColor(190, 160, 255),
        TermColor(140, 230, 240),
        TermColor(240, 248, 255),
    ];
    if index < 16 {
        return ANSI[index as usize];
    }
    if (16..=231).contains(&index) {
        let idx = index - 16;
        let r = idx / 36;
        let g = (idx % 36) / 6;
        let b = idx % 6;
        let convert = |v: u8| if v == 0 { 0 } else { 55 + v * 40 };
        return TermColor(convert(r), convert(g), convert(b));
    }
    let gray = 8 + (index.saturating_sub(232)) * 10;
    TermColor(gray, gray, gray)
}

#[component]
pub fn AlacrittyTerminal(props: AlacrittyTerminalProps) -> Element {
    let rows = props.rows;
    let cols = props.cols;
    let command = props.command.clone();
    let args = props.args.clone();
    let session = use_signal(move || {
        Arc::new(Mutex::new(
            AlacrittyTerminalSession::spawn(&command, &args, rows, cols).ok(),
        ))
    });
    let mut snapshot = use_signal(|| TerminalSnapshot::blank(rows, cols));

    {
        let session = session.clone();
        use_future(move || async move {
            loop {
                let next = {
                    let session_arc = session.read().clone();
                    let mut guard = session_arc.lock().unwrap();
                    if let Some(ref mut session) = *guard {
                        if session.poll() {
                            Some(session.snapshot(rows, cols))
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                };
                if let Some(next) = next {
                    snapshot.set(next);
                }
                tokio::time::sleep(Duration::from_millis(16)).await;
            }
        });
    }

    let onkeydown = {
        let session = session.clone();
        move |evt: KeyboardEvent| {
            let input = key_to_terminal_input(&evt);
            if input.is_empty() {
                return;
            }
            if let Ok(guard) = session.read().lock() {
                if let Some(ref session) = *guard {
                    session.write_input(&input);
                }
            }
        }
    };

    let font_family = "JetBrainsMono Nerd Font, JetBrains Mono, FiraCode Nerd Font, Fira Code, MesloLGS NF, Symbols Nerd Font Mono, Symbols Nerd Font, SF Mono, Menlo, Monaco, Cascadia Code, Consolas, ui-monospace, monospace";

    rsx! {
        div {
            class: "flynt-alacritty-terminal {props.class}",
            tabindex: "0",
            onkeydown,
            style: "background: {TermColor::BG.css()}; color: {TermColor::FG.css()}; font-family: {font_family}; font-size: {props.font_size}px; line-height: 1.2; overflow: hidden; white-space: pre;",
            for (row_idx, row) in snapshot.read().rows.iter().enumerate() {
                div { key: "row-{row_idx}", class: "flynt-terminal-row",
                    for (col_idx, cell) in row.iter().enumerate() {
                        {
                            let is_cursor = snapshot.read().cursor == (row_idx, col_idx);
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

fn key_to_terminal_input(evt: &KeyboardEvent) -> String {
    match evt.key() {
        Key::Enter => "\r".to_string(),
        Key::Backspace => "\x7f".to_string(),
        Key::Tab => "\t".to_string(),
        Key::Escape => "\x1b".to_string(),
        Key::ArrowUp => "\x1b[A".to_string(),
        Key::ArrowDown => "\x1b[B".to_string(),
        Key::ArrowRight => "\x1b[C".to_string(),
        Key::ArrowLeft => "\x1b[D".to_string(),
        Key::Character(s) if evt.modifiers().ctrl() && s.len() == 1 => {
            let b = s.as_bytes()[0].to_ascii_uppercase();
            if b.is_ascii_alphabetic() {
                ((b - b'A' + 1) as char).to_string()
            } else {
                String::new()
            }
        }
        Key::Character(s) => s,
        _ => String::new(),
    }
}
