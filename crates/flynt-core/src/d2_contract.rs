use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum D2LintSeverity {
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct D2LintDiagnostic {
    pub severity: D2LintSeverity,
    pub message: String,
}

pub fn lint_d2_source(source: &str) -> Vec<D2LintDiagnostic> {
    let mut diagnostics = Vec::new();
    let mut lines = source.lines().enumerate().peekable();

    while let Some((idx, line)) = lines.next() {
        let trimmed = line.trim();
        if trimmed.ends_with("|md") || trimmed.contains(": |md") {
            let label = trimmed.split(':').next().unwrap_or("").trim();
            let mut body_lines = 0usize;
            for (_, body) in lines.by_ref() {
                if body.trim() == "|" || body.trim().starts_with("| {") {
                    break;
                }
                if !body.trim().is_empty() {
                    body_lines += 1;
                }
            }
            if !label.eq_ignore_ascii_case("title") && body_lines > 2 {
                diagnostics.push(D2LintDiagnostic {
                    severity: D2LintSeverity::Warning,
                    message: format!(
                        "line {}: multiline |md node `{label}` has {body_lines} content lines; use child nodes instead to avoid clipped SVG foreignObject text",
                        idx + 1
                    ),
                });
            }
        }

        if trimmed.contains("->") {
            let label = trimmed
                .split_once(':')
                .map(|(_, label)| label.split('{').next().unwrap_or(label).trim())
                .unwrap_or("");
            if label.split_whitespace().count() > 4 {
                diagnostics.push(D2LintDiagnostic {
                    severity: D2LintSeverity::Warning,
                    message: format!(
                        "line {}: long edge label `{label}`; keep labels short and move explanation into a note node",
                        idx + 1
                    ),
                });
            }
        }
    }

    diagnostics
}

pub fn lint_d2_file(path: &Path) -> std::io::Result<Vec<D2LintDiagnostic>> {
    let source = std::fs::read_to_string(path)?;
    Ok(lint_d2_source(&source))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn warns_on_multiline_markdown_nodes() {
        let source = r#"
queue: |md
  **Command Queue**
  durable queue
  monotonic sequence cursors
  atomic with writes
|
"#;
        let diagnostics = lint_d2_source(source);
        assert!(
            diagnostics
                .iter()
                .any(|d| d.message.contains("multiline |md node"))
        );
    }

    #[test]
    fn allows_title_markdown() {
        let source = r#"
title: |md
  # Architecture
  compact subtitle
  one more line
|
"#;
        assert!(lint_d2_source(source).is_empty());
    }

    #[test]
    fn warns_on_long_edge_labels() {
        let source = "api -> queue: writes command intent atomically with business state";
        let diagnostics = lint_d2_source(source);
        assert!(
            diagnostics
                .iter()
                .any(|d| d.message.contains("long edge label"))
        );
    }
}
