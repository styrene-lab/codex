use crate::models::Frontmatter;
use crate::parser::parse_document_source;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

/// Semantic report model for formal presentation exports.
///
/// Flynt has two Markdown regimes:
/// - plain Markdown notes keep conservative note semantics;
/// - Report Markdown (`kind = "report"` or a `[report]` table) opts into a
///   compiled-document surface with Typst-backed presentation semantics.
///
/// Report generation compiles source into this Flynt-owned IR first, then
/// renders a Typst bundle from the IR. Keeping the IR independent from Typst
/// preserves Flynt semantics such as frontmatter, wikilinks, provenance, and
/// future artifact embeds while still allowing Typst-aware report constructs.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Report {
    pub title: String,
    pub source_path: PathBuf,
    pub generated_at: DateTime<Utc>,
    pub frontmatter: Frontmatter,
    pub config: ReportConfig,
    pub diagnostics: Vec<ReportDiagnostic>,
    pub blocks: Vec<ReportBlock>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportConfig {
    pub source_mode: ReportSourceMode,
    pub backend: ReportBackend,
    pub profile: String,
    pub math_mode: MathMode,
    pub unicode_mode: UnicodeMode,
    pub raw_typst: RawTypstPolicy,
    pub toc: bool,
    pub number_headings: bool,
}

impl Default for ReportConfig {
    fn default() -> Self {
        Self {
            source_mode: ReportSourceMode::PlainMarkdown,
            backend: ReportBackend::Typst,
            profile: "technical-brief".to_string(),
            math_mode: MathMode::Literal,
            unicode_mode: UnicodeMode::Literal,
            raw_typst: RawTypstPolicy::Deny,
            toc: true,
            number_headings: true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportSourceMode {
    PlainMarkdown,
    ReportMarkdown,
    NativeTypst,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportBackend {
    Typst,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MathMode {
    Off,
    Literal,
    Typst,
    LatexWarn,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnicodeMode {
    Literal,
    Diagnose,
    Strict,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RawTypstPolicy {
    Deny,
    Allow,
    TrustedOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportDiagnostic {
    pub severity: DiagnosticSeverity,
    pub code: String,
    pub message: String,
    pub span: Option<SourceSpan>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticSeverity {
    Info,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceSpan {
    pub path: PathBuf,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ReportBlock {
    Heading {
        level: u8,
        text: String,
        span: SourceSpan,
    },
    Paragraph {
        text: String,
        span: SourceSpan,
    },
    CodeBlock {
        language: Option<String>,
        text: String,
        span: SourceSpan,
    },
    TypstMathBlock {
        text: String,
        span: SourceSpan,
    },
    RawTypstBlock {
        text: String,
        span: SourceSpan,
    },
    ReportDirective {
        directive: String,
        span: SourceSpan,
    },
    ThematicBreak {
        span: SourceSpan,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReportBundleManifest {
    pub title: String,
    pub source_path: PathBuf,
    pub source_sha256: String,
    pub generated_at: DateTime<Utc>,
    pub typst_path: PathBuf,
    pub pdf_path: Option<PathBuf>,
    pub backend: String,
    pub source_mode: ReportSourceMode,
    pub profile: String,
    pub diagnostics: Vec<ReportDiagnostic>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReportBundle {
    pub report_typ: PathBuf,
    pub manifest_json: PathBuf,
    pub pdf: Option<PathBuf>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormalizeResult {
    pub source_note: PathBuf,
    pub document_path: PathBuf,
    pub manifest_path: PathBuf,
    pub title: String,
    pub source_sha256: String,
    pub diagnostics: Vec<ReportDiagnostic>,
}

pub fn is_report_markdown(frontmatter: &Frontmatter, raw_markdown: &str) -> bool {
    frontmatter.kind.as_deref() == Some("report") || raw_report_table(raw_markdown).is_some()
}

pub fn report_from_markdown_source(
    source_path: impl Into<PathBuf>,
    raw_markdown: &str,
    generated_at: DateTime<Utc>,
) -> Report {
    let source_path = source_path.into();
    let (body, frontmatter, _) = parse_document_source(raw_markdown);
    let mut config = report_config_from_source(&frontmatter, raw_markdown);
    if source_path.extension().and_then(|e| e.to_str()) == Some("typ") {
        config.source_mode = ReportSourceMode::NativeTypst;
    }
    let title = frontmatter
        .title
        .clone()
        .or_else(|| first_heading(&body))
        .or_else(|| {
            source_path
                .file_stem()
                .map(|s| s.to_string_lossy().to_string())
        })
        .unwrap_or_else(|| "Flynt Report".to_string());

    let (blocks, mut diagnostics) = parse_report_blocks(&body, &source_path, &config);
    diagnostics.extend(scan_report_diagnostics(&body, &source_path, &config));

    Report {
        title,
        source_path,
        generated_at,
        frontmatter,
        config,
        diagnostics,
        blocks,
    }
}

pub fn render_typst_report(report: &Report) -> String {
    let mut out = String::new();
    out.push_str("// Generated by Flynt. Edit the source markdown, not this file.\n");
    out.push_str("#set document(title: ");
    out.push_str(&typst_string(&report.title));
    out.push_str(")\n");
    out.push_str("#set page(paper: \"us-letter\", margin: (x: 0.8in, y: 0.75in))\n");
    out.push_str("#set text(font: \"New Computer Modern\", size: 10.5pt)\n");
    if report.config.number_headings {
        out.push_str("#set heading(numbering: \"1.1\")\n");
    }
    out.push_str("#show heading.where(level: 1): it => block(above: 1.2em, below: 0.7em, text(16pt, weight: \"bold\", it))\n");
    out.push_str("#show heading.where(level: 2): it => block(above: 1.0em, below: 0.45em, text(13pt, weight: \"bold\", it))\n");
    out.push_str("#show raw.where(block: true): it => block(fill: rgb(\"f6f8fa\"), inset: 8pt, radius: 3pt, width: 100%, it)\n\n");
    out.push_str("#align(center)[\n");
    out.push_str("  #text(22pt, weight: \"bold\")[");
    out.push_str(&typst_markup_text(&report.title));
    out.push_str("]\\\n");
    out.push_str("  #text(8pt, fill: gray)[Compiled from `");
    out.push_str(&typst_markup_text(&report.source_path.to_string_lossy()));
    out.push_str("`]\n");
    out.push_str("]\n\n");
    if report.config.toc {
        out.push_str("#outline(title: [Contents])\n\n");
    }

    for block in &report.blocks {
        match block {
            ReportBlock::Heading { level, text, .. } => {
                out.push_str(&"=".repeat((*level).clamp(1, 6) as usize));
                out.push(' ');
                out.push_str(&typst_markup_text(text));
                out.push_str("\n\n");
            }
            ReportBlock::Paragraph { text, .. } => {
                out.push_str(&render_paragraph_typst(text, report.config.math_mode));
                out.push_str("\n\n");
            }
            ReportBlock::CodeBlock { language, text, .. } => {
                out.push_str("``` ");
                if let Some(language) = language {
                    out.push_str(&typst_markup_text(language));
                }
                out.push('\n');
                out.push_str(text.trim_end());
                out.push_str("\n```\n\n");
            }
            ReportBlock::TypstMathBlock { text, .. } => {
                out.push_str("$ ");
                out.push_str(text.trim());
                out.push_str(" $\n\n");
            }
            ReportBlock::RawTypstBlock { text, .. } => {
                if report.config.raw_typst == RawTypstPolicy::Allow {
                    out.push_str(text.trim_end());
                    out.push_str("\n\n");
                } else {
                    out.push_str("``` typst\n");
                    out.push_str(text.trim_end());
                    out.push_str("\n```\n\n");
                }
            }
            ReportBlock::ReportDirective { directive, .. } => {
                if directive.trim() == "page-break" {
                    out.push_str("#pagebreak()\n\n");
                }
            }
            ReportBlock::ThematicBreak { .. } => out.push_str("#line(length: 100%)\n\n"),
        }
    }

    out
}

pub fn formalize_markdown_note(
    source_path: impl AsRef<Path>,
    document_path: impl AsRef<Path>,
    manifest_path: impl AsRef<Path>,
) -> Result<FormalizeResult> {
    let source_path = source_path.as_ref();
    let document_path = document_path.as_ref();
    let manifest_path = manifest_path.as_ref();
    let raw = fs::read_to_string(source_path)
        .with_context(|| format!("read note source {}", source_path.display()))?;
    let generated_at = Utc::now();
    let report = report_from_markdown_source(source_path.to_path_buf(), &raw, generated_at);
    if let Some(parent) = document_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create document dir {}", parent.display()))?;
    }
    if let Some(parent) = manifest_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create manifest dir {}", parent.display()))?;
    }

    let typst = render_formalized_typst_document(&report, &raw);
    fs::write(document_path, typst)
        .with_context(|| format!("write formal document {}", document_path.display()))?;

    let result = FormalizeResult {
        source_note: source_path.to_path_buf(),
        document_path: document_path.to_path_buf(),
        manifest_path: manifest_path.to_path_buf(),
        title: report.title,
        source_sha256: sha256_hex(raw.as_bytes()),
        diagnostics: report.diagnostics,
    };
    fs::write(manifest_path, serde_json::to_string_pretty(&result)?)
        .with_context(|| format!("write formalization manifest {}", manifest_path.display()))?;
    Ok(result)
}

pub fn compile_markdown_report(
    source_path: impl AsRef<Path>,
    output_dir: impl AsRef<Path>,
    run_typst: bool,
) -> Result<ReportBundle> {
    let source_path = source_path.as_ref();
    let output_dir = output_dir.as_ref();
    let raw = fs::read_to_string(source_path)
        .with_context(|| format!("read report source {}", source_path.display()))?;
    let generated_at = Utc::now();
    let report = report_from_markdown_source(source_path.to_path_buf(), &raw, generated_at);
    fs::create_dir_all(output_dir)
        .with_context(|| format!("create report output dir {}", output_dir.display()))?;

    let report_typ = output_dir.join("report.typ");
    let manifest_json = output_dir.join("manifest.json");
    let pdf_path = output_dir.join("report.pdf");
    fs::write(&report_typ, render_typst_report(&report))
        .with_context(|| format!("write {}", report_typ.display()))?;

    let mut pdf = None;
    if run_typst {
        let status = Command::new("typst")
            .arg("compile")
            .arg(&report_typ)
            .arg(&pdf_path)
            .status()
            .context("run typst compile; install typst or build with run_typst=false")?;
        if !status.success() {
            anyhow::bail!("typst compile failed with status {status}");
        }
        pdf = Some(pdf_path.clone());
    }

    let manifest = ReportBundleManifest {
        title: report.title,
        source_path: source_path.to_path_buf(),
        source_sha256: sha256_hex(raw.as_bytes()),
        generated_at,
        typst_path: report_typ.clone(),
        pdf_path: pdf.clone(),
        backend: "typst".to_string(),
        source_mode: report.config.source_mode,
        profile: report.config.profile,
        diagnostics: report.diagnostics,
    };
    fs::write(&manifest_json, serde_json::to_string_pretty(&manifest)?)
        .with_context(|| format!("write {}", manifest_json.display()))?;

    Ok(ReportBundle {
        report_typ,
        manifest_json,
        pdf,
    })
}

fn render_formalized_typst_document(report: &Report, raw_markdown: &str) -> String {
    let mut out = String::new();
    out.push_str("// Formalized by Flynt from a Markdown note.\n");
    out.push_str(
        "// After formalization, this .typ file is the canonical formal document source.\n",
    );
    out.push_str("// Source note: ");
    out.push_str(&report.source_path.to_string_lossy());
    out.push('\n');
    out.push_str("// Source SHA-256: ");
    out.push_str(&sha256_hex(raw_markdown.as_bytes()));
    out.push_str("\n\n");
    out.push_str("#set document(title: ");
    out.push_str(&typst_string(&report.title));
    out.push_str(")\n");
    out.push_str("#set page(paper: \"us-letter\", margin: (x: 0.8in, y: 0.75in))\n");
    out.push_str("#set text(font: \"New Computer Modern\", size: 10.5pt)\n");
    out.push_str("#set heading(numbering: \"1.1\")\n\n");

    for block in &report.blocks {
        match block {
            ReportBlock::Heading { level, text, .. } => {
                out.push_str(&"=".repeat((*level).clamp(1, 6) as usize));
                out.push(' ');
                out.push_str(&typst_markup_text(text));
                out.push_str("\n\n");
            }
            ReportBlock::Paragraph { text, .. } => {
                out.push_str(&typst_markup_text(text));
                out.push_str("\n\n");
            }
            ReportBlock::CodeBlock { language, text, .. } => {
                out.push_str("``` ");
                if let Some(language) = language {
                    out.push_str(&typst_markup_text(language));
                }
                out.push('\n');
                out.push_str(text.trim_end());
                out.push_str("\n```\n\n");
            }
            ReportBlock::TypstMathBlock { text, .. } => {
                out.push_str("$ ");
                out.push_str(text.trim());
                out.push_str(" $\n\n");
            }
            ReportBlock::RawTypstBlock { text, .. } => {
                out.push_str(text.trim_end());
                out.push_str("\n\n");
            }
            ReportBlock::ReportDirective { directive, .. } => {
                if directive.trim() == "page-break" {
                    out.push_str("#pagebreak()\n\n");
                }
            }
            ReportBlock::ThematicBreak { .. } => out.push_str("#line(length: 100%)\n\n"),
        }
    }
    out
}

fn report_config_from_source(frontmatter: &Frontmatter, raw: &str) -> ReportConfig {
    let mut config = ReportConfig::default();
    if is_report_markdown(frontmatter, raw) {
        config.source_mode = ReportSourceMode::ReportMarkdown;
        config.unicode_mode = UnicodeMode::Diagnose;
    }
    let Some(table) = raw_report_table(raw) else {
        return config;
    };
    if let Some(value) = table.get("profile").and_then(toml::Value::as_str) {
        config.profile = value.to_string();
    }
    if let Some(value) = table
        .get("math")
        .or_else(|| table.get("math_mode"))
        .and_then(toml::Value::as_str)
    {
        config.math_mode = match value {
            "off" => MathMode::Off,
            "typst" => MathMode::Typst,
            "latex_warn" | "latex-warn" => MathMode::LatexWarn,
            _ => MathMode::Literal,
        };
    }
    if let Some(value) = table
        .get("unicode")
        .or_else(|| table.get("unicode_mode"))
        .and_then(toml::Value::as_str)
    {
        config.unicode_mode = match value {
            "strict" => UnicodeMode::Strict,
            "diagnose" => UnicodeMode::Diagnose,
            _ => UnicodeMode::Literal,
        };
    }
    if let Some(value) = table.get("raw_typst").and_then(toml::Value::as_str) {
        config.raw_typst = match value {
            "allow" => RawTypstPolicy::Allow,
            "trusted_only" | "trusted-only" => RawTypstPolicy::TrustedOnly,
            _ => RawTypstPolicy::Deny,
        };
    }
    if let Some(value) = table.get("toc").and_then(toml::Value::as_bool) {
        config.toc = value;
    }
    if let Some(value) = table.get("number_headings").and_then(toml::Value::as_bool) {
        config.number_headings = value;
    }
    config
}

fn raw_report_table(raw: &str) -> Option<toml::value::Table> {
    let rest = raw.strip_prefix("+++\n")?;
    let end = rest.find("\n+++")?;
    let fm_str = &rest[..end];
    let value: toml::Value = toml::from_str(fm_str).ok()?;
    value.get("report")?.as_table().cloned()
}

fn parse_report_blocks(
    body: &str,
    source_path: &Path,
    config: &ReportConfig,
) -> (Vec<ReportBlock>, Vec<ReportDiagnostic>) {
    let mut blocks = Vec::new();
    let mut diagnostics = Vec::new();
    let mut paragraph = Vec::<(usize, String)>::new();
    let lines: Vec<&str> = body.lines().collect();
    let mut idx = 0;

    while idx < lines.len() {
        let line_no = idx + 1;
        let line = lines[idx];
        if let Some(rest) = line.strip_prefix("```") {
            flush_paragraph(&mut blocks, &mut paragraph, source_path);
            let language = rest.trim();
            let start = line_no;
            idx += 1;
            let mut text = String::new();
            while idx < lines.len() && !lines[idx].starts_with("```") {
                text.push_str(lines[idx]);
                text.push('\n');
                idx += 1;
            }
            let end = (idx + 1).min(lines.len());
            let span = span(source_path, start, end);
            match language {
                "typst-math" => blocks.push(ReportBlock::TypstMathBlock { text, span }),
                "typst" => {
                    if config.raw_typst != RawTypstPolicy::Allow {
                        diagnostics.push(ReportDiagnostic {
                            severity: DiagnosticSeverity::Warning,
                            code: "raw_typst_block".to_string(),
                            message: "Raw Typst block is present but raw_typst is not set to allow; it will render as a code block.".to_string(),
                            span: Some(span.clone()),
                        });
                    }
                    blocks.push(ReportBlock::RawTypstBlock { text, span });
                }
                "report" => blocks.push(ReportBlock::ReportDirective {
                    directive: text.trim().to_string(),
                    span,
                }),
                _ => blocks.push(ReportBlock::CodeBlock {
                    language: (!language.is_empty()).then(|| language.to_string()),
                    text,
                    span,
                }),
            }
            idx += 1;
            continue;
        }

        if line.trim().is_empty() {
            flush_paragraph(&mut blocks, &mut paragraph, source_path);
            idx += 1;
            continue;
        }

        if matches!(line.trim(), "---" | "***" | "___") {
            flush_paragraph(&mut blocks, &mut paragraph, source_path);
            blocks.push(ReportBlock::ThematicBreak {
                span: span(source_path, line_no, line_no),
            });
            idx += 1;
            continue;
        }

        if let Some((level, text)) = markdown_heading(line) {
            flush_paragraph(&mut blocks, &mut paragraph, source_path);
            blocks.push(ReportBlock::Heading {
                level,
                text,
                span: span(source_path, line_no, line_no),
            });
            idx += 1;
            continue;
        }

        paragraph.push((line_no, line.trim().to_string()));
        idx += 1;
    }

    flush_paragraph(&mut blocks, &mut paragraph, source_path);
    (blocks, diagnostics)
}

fn flush_paragraph(
    blocks: &mut Vec<ReportBlock>,
    paragraph: &mut Vec<(usize, String)>,
    source_path: &Path,
) {
    if paragraph.is_empty() {
        return;
    }
    let start = paragraph.first().map(|(line, _)| *line).unwrap_or(1);
    let end = paragraph.last().map(|(line, _)| *line).unwrap_or(start);
    let text = paragraph
        .iter()
        .map(|(_, text)| text.as_str())
        .collect::<Vec<_>>()
        .join(" ");
    blocks.push(ReportBlock::Paragraph {
        text,
        span: span(source_path, start, end),
    });
    paragraph.clear();
}

fn scan_report_diagnostics(
    body: &str,
    source_path: &Path,
    config: &ReportConfig,
) -> Vec<ReportDiagnostic> {
    let mut diagnostics = Vec::new();
    for (idx, line) in body.lines().enumerate() {
        let line_no = idx + 1;
        if config.source_mode == ReportSourceMode::PlainMarkdown && line.contains('$') {
            diagnostics.push(ReportDiagnostic {
                severity: DiagnosticSeverity::Info,
                code: "plain_markdown_dollar_literal".to_string(),
                message: "Plain Markdown keeps dollar signs literal; convert to Report Markdown to opt into Typst math.".to_string(),
                span: Some(span(source_path, line_no, line_no)),
            });
        } else if matches!(config.math_mode, MathMode::LatexWarn)
            && line.contains('\\')
            && line.contains('$')
        {
            diagnostics.push(ReportDiagnostic {
                severity: DiagnosticSeverity::Warning,
                code: "latex_math_in_typst_report".to_string(),
                message: "This looks like LaTeX math; Typst math is not LaTeX-compatible. Use typst-math or convert explicitly.".to_string(),
                span: Some(span(source_path, line_no, line_no)),
            });
        }
        if matches!(
            config.unicode_mode,
            UnicodeMode::Diagnose | UnicodeMode::Strict
        ) && line.chars().any(|c| !c.is_ascii())
        {
            diagnostics.push(ReportDiagnostic {
                severity: if config.unicode_mode == UnicodeMode::Strict {
                    DiagnosticSeverity::Error
                } else {
                    DiagnosticSeverity::Warning
                },
                code: "unicode_glyph_check".to_string(),
                message: "Non-ASCII characters need Typst/font preview before publication."
                    .to_string(),
                span: Some(span(source_path, line_no, line_no)),
            });
        }
    }
    diagnostics
}

fn render_paragraph_typst(text: &str, math_mode: MathMode) -> String {
    match math_mode {
        MathMode::Typst => render_typst_math_spans(text),
        _ => typst_markup_text(text),
    }
}

fn render_typst_math_spans(text: &str) -> String {
    let mut out = String::new();
    let mut rest = text;
    let mut math = false;
    while let Some(pos) = rest.find('$') {
        let (before, after) = rest.split_at(pos);
        if math {
            out.push_str(before);
            out.push('$');
        } else {
            out.push_str(&typst_markup_text(before));
            out.push('$');
        }
        math = !math;
        rest = &after[1..];
    }
    if math {
        out.push_str(&typst_markup_text(rest));
    } else {
        out.push_str(&typst_markup_text(rest));
    }
    out
}

fn span(source_path: &Path, start_line: usize, end_line: usize) -> SourceSpan {
    SourceSpan {
        path: source_path.to_path_buf(),
        start_line,
        end_line,
    }
}

fn markdown_heading(line: &str) -> Option<(u8, String)> {
    let trimmed = line.trim_start();
    let level = trimmed.chars().take_while(|c| *c == '#').count();
    if !(1..=6).contains(&level) {
        return None;
    }
    let rest = trimmed[level..].trim_start();
    if rest.is_empty() {
        return None;
    }
    Some((level as u8, rest.trim_end_matches('#').trim().to_string()))
}

fn first_heading(body: &str) -> Option<String> {
    body.lines()
        .find_map(markdown_heading)
        .map(|(_, text)| text)
}

fn typst_string(value: &str) -> String {
    let escaped = value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n");
    format!("\"{escaped}\"")
}

fn typst_markup_text(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('[', "\\[")
        .replace(']', "\\]")
        .replace('#', "\\#")
        .replace('$', "\\$")
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    #[test]
    fn builds_report_ir_from_frontmatter_and_markdown() {
        let raw = "+++\ntitle = \"Formal Brief\"\ntags = [\"report\"]\n+++\n\n# Intro\n\nBody text.\n\n```rust\nfn main() {}\n```\n";
        let report = report_from_markdown_source(
            "docs/brief.md",
            raw,
            Utc.with_ymd_and_hms(2026, 6, 29, 0, 0, 0).unwrap(),
        );

        assert_eq!(report.title, "Formal Brief");
        assert!(
            matches!(report.blocks[0], ReportBlock::Heading { level: 1, ref text, .. } if text == "Intro")
        );
        assert!(
            matches!(report.blocks[2], ReportBlock::CodeBlock { ref language, .. } if language.as_deref() == Some("rust"))
        );
    }

    #[test]
    fn report_frontmatter_opts_into_report_markdown_and_typst_math() {
        let raw = "+++\ntitle = \"Formal Brief\"\nkind = \"report\"\n[report]\nmath = \"typst\"\nraw_typst = \"allow\"\n+++\n\n# Intro\n\nEnergy is $E = m c^2$.\n\n```typst-math\nsum_(i=1)^n i\n```\n";
        let report = report_from_markdown_source(
            "docs/brief.report.md",
            raw,
            Utc.with_ymd_and_hms(2026, 6, 29, 0, 0, 0).unwrap(),
        );

        assert_eq!(report.config.source_mode, ReportSourceMode::ReportMarkdown);
        assert_eq!(report.config.math_mode, MathMode::Typst);
        assert!(matches!(
            report.blocks[2],
            ReportBlock::TypstMathBlock { .. }
        ));
        let typst = render_typst_report(&report);
        assert!(typst.contains("Energy is $E = m c^2$."));
        assert!(typst.contains("$ sum_(i=1)^n i $"));
    }

    #[test]
    fn plain_markdown_keeps_dollar_math_literal_and_reports_diagnostic() {
        let report = report_from_markdown_source(
            "docs/plain.md",
            "# Intro\n\nCost is $5 and energy is $E$.",
            Utc.with_ymd_and_hms(2026, 6, 29, 0, 0, 0).unwrap(),
        );

        assert_eq!(report.config.source_mode, ReportSourceMode::PlainMarkdown);
        assert!(
            report
                .diagnostics
                .iter()
                .any(|d| d.code == "plain_markdown_dollar_literal")
        );
        let typst = render_typst_report(&report);
        assert!(typst.contains("Cost is \\$5 and energy is \\$E\\$."));
    }

    #[test]
    fn raw_typst_is_warned_unless_allowed() {
        let raw = "+++\nkind = \"report\"\n[report]\nraw_typst = \"deny\"\n+++\n\n```typst\n#pagebreak()\n```\n";
        let report = report_from_markdown_source(
            "docs/raw.report.md",
            raw,
            Utc.with_ymd_and_hms(2026, 6, 29, 0, 0, 0).unwrap(),
        );
        assert!(
            report
                .diagnostics
                .iter()
                .any(|d| d.code == "raw_typst_block")
        );
        let typst = render_typst_report(&report);
        assert!(typst.contains("``` typst"));
    }

    #[test]
    fn formalizes_markdown_note_to_typst_document_with_manifest() {
        let dir = tempfile::tempdir().unwrap();
        let source = dir.path().join("note.md");
        let document = dir.path().join("documents/note.typ");
        let manifest = dir.path().join(".flynt/formalizations/note.json");
        fs::write(
            &source,
            "+++\ntitle = \"Formal Me\"\n+++\n\n# Formal Me\n\nA paragraph.\n",
        )
        .unwrap();

        let result = formalize_markdown_note(&source, &document, &manifest).unwrap();

        assert_eq!(result.title, "Formal Me");
        assert!(document.exists());
        assert!(manifest.exists());
        let typst = fs::read_to_string(document).unwrap();
        assert!(typst.contains(
            "After formalization, this .typ file is the canonical formal document source."
        ));
        assert!(typst.contains("= Formal Me"));
        assert!(typst.contains("A paragraph."));
    }

    #[test]
    fn renders_typst_bundle_with_report_metadata() {
        let report = report_from_markdown_source(
            "docs/brief.md",
            "# Intro\n\nA paragraph with [brackets] and #hash.",
            Utc.with_ymd_and_hms(2026, 6, 29, 0, 0, 0).unwrap(),
        );
        let typst = render_typst_report(&report);

        assert!(typst.contains("#set document(title: \"Intro\")"));
        assert!(typst.contains("= Intro"));
        assert!(typst.contains("\\[brackets\\]"));
        assert!(typst.contains("\\#hash"));
    }
}
