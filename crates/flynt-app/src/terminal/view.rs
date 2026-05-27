//! Reusable terminal substrate using portable-pty and alacritty_terminal.
//!
//! This intentionally avoids `dioxus-terminal`: that crate proved useful for
//! PTY/layout exploration, but its renderer casts UTF-8 bytes directly to
//! chars. Keep this module free of Flynt-specific policy so it can be extracted
//! for Auspex or a shared Styrene terminal crate.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;

use dioxus::prelude::ModifiersInteraction;

use alacritty_terminal::event::VoidListener;
use alacritty_terminal::index::Point;
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::test::TermSize;
use alacritty_terminal::term::Term;
use alacritty_terminal::vte::ansi::{Color as VteColor, NamedColor, Processor, Rgb};
use portable_pty::{Child, CommandBuilder, ExitStatus, PtySize, native_pty_system};
use tokio::sync::mpsc;

use super::render::{RenderCell, TermColor};
use super::types::{TerminalCreateParams, TerminalStatus};

pub const DEFAULT_ROWS: usize = 34;
pub const DEFAULT_COLS: usize = 120;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TerminalSnapshot {
    pub(crate) rows: Vec<Vec<RenderCell>>,
    pub(crate) cursor: (usize, usize),
}

impl TerminalSnapshot {
    pub fn blank(rows: usize, cols: usize) -> Self {
        Self {
            rows: vec![vec![RenderCell::default(); cols]; rows],
            cursor: (0, 0),
        }
    }
}

struct PtyHandle {
    writer: Arc<Mutex<Box<dyn Write + Send>>>,
    rx: mpsc::Receiver<Vec<u8>>,
    child: Arc<Mutex<Box<dyn Child + Send + Sync>>>,
    master: Arc<Mutex<Box<dyn portable_pty::MasterPty + Send>>>,
}

pub struct AlacrittyTerminalSession {
    term: Term<VoidListener>,
    processor: Processor,
    pty: PtyHandle,
}

impl AlacrittyTerminalSession {
    pub fn spawn(command: &str, args: &[String], rows: usize, cols: usize) -> anyhow::Result<Self> {
        Self::spawn_with_params(
            TerminalCreateParams::new(command.to_string()).with_args(args.iter().cloned()),
            None,
            rows,
            cols,
        )
    }

    pub fn spawn_with_params(
        params: TerminalCreateParams,
        fallback_cwd: Option<&Path>,
        rows: usize,
        cols: usize,
    ) -> anyhow::Result<Self> {
        let size = PtySize {
            rows: rows as u16,
            cols: cols as u16,
            pixel_width: 0,
            pixel_height: 0,
        };
        let pair = native_pty_system().openpty(size)?;
        let mut cmd = CommandBuilder::new(&params.command);
        cmd.args(params.args.iter().map(String::as_str));
        if let Some(cwd) = params.cwd.as_deref() {
            let cwd = PathBuf::from(cwd);
            cmd.cwd(cwd.as_os_str());
        } else if let Some(cwd) = fallback_cwd {
            cmd.cwd(cwd.as_os_str());
        }
        for (key, value) in &params.env {
            cmd.env(key, value);
        }
        let child = pair.slave.spawn_command(cmd)?;

        let writer = Arc::new(Mutex::new(pair.master.take_writer()?));
        let mut reader = pair.master.try_clone_reader()?;
        let master = Arc::new(Mutex::new(pair.master));
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
            pty: PtyHandle {
                writer,
                rx,
                child: Arc::new(Mutex::new(child)),
                master,
            },
        })
    }

    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        while let Ok(bytes) = self.pty.rx.try_recv() {
            self.processor.advance(&mut self.term, &bytes);
            changed = true;
        }
        changed
    }

    pub fn write_input(&self, input: &str) {
        if let Ok(mut writer) = self.pty.writer.lock() {
            let _ = writer.write_all(input.as_bytes());
            let _ = writer.flush();
        }
    }

    pub fn resize(&mut self, rows: usize, cols: usize) -> anyhow::Result<()> {
        let rows = rows.max(1) as u16;
        let cols = cols.max(1) as u16;
        self.term.resize(TermSize::new(cols as usize, rows as usize));
        self.pty.master.lock().unwrap().resize(PtySize {
            rows,
            cols,
            pixel_width: 0,
            pixel_height: 0,
        })?;
        Ok(())
    }

    pub fn snapshot(&self, rows: usize, cols: usize) -> TerminalSnapshot {
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
    pub fn try_wait_status(&self) -> TerminalStatus {
        let mut child = self.pty.child.lock().unwrap();
        match child.try_wait() {
            Ok(Some(status)) => {
                TerminalStatus::Exited(exit_status_label(&status))
            }
            Ok(None) => TerminalStatus::Running,
            Err(err) => TerminalStatus::Failed(err.to_string()),
        }
    }

    pub fn kill(&self) -> anyhow::Result<()> {
        self.pty.child.lock().unwrap().kill()?;
        Ok(())
    }

}


fn exit_status_label(status: &ExitStatus) -> String {
    if let Some(signal) = status.signal() {
        format!("signal {signal}")
    } else {
        format!("exit code {}", status.exit_code())
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

pub(crate) fn key_to_terminal_input(evt: &dioxus::prelude::KeyboardEvent) -> String {
    match evt.key() {
        dioxus::prelude::Key::Enter => "\r".to_string(),
        dioxus::prelude::Key::Backspace => "\x7f".to_string(),
        dioxus::prelude::Key::Tab => "\t".to_string(),
        dioxus::prelude::Key::Escape => "\x1b".to_string(),
        dioxus::prelude::Key::ArrowUp => "\x1b[A".to_string(),
        dioxus::prelude::Key::ArrowDown => "\x1b[B".to_string(),
        dioxus::prelude::Key::ArrowRight => "\x1b[C".to_string(),
        dioxus::prelude::Key::ArrowLeft => "\x1b[D".to_string(),
        dioxus::prelude::Key::Character(s) if evt.modifiers().ctrl() && s.len() == 1 => {
            let b = s.as_bytes()[0].to_ascii_uppercase();
            if b.is_ascii_alphabetic() {
                ((b - b'A' + 1) as char).to_string()
            } else {
                String::new()
            }
        }
        dioxus::prelude::Key::Character(s) => s,
        _ => String::new(),
    }
}
