use crate::components::{FloatingNotePreview, NotePreview};
use crate::{
    bootstrap::AppContext,
    state::{
        NoteHistoryCommand, NoteInspectorCommand, NoteInspectorTarget, PublicationPreviewCommand,
        Route, TabState,
    },
};
use comrak::{Options, markdown_to_html};
use dioxus::prelude::*;
use flynt_core::models::{
    DocumentId, DocumentMeta, Frontmatter, PublicationConfig, PublicationVisibility,
};
use flynt_core::parser::parse_document_source;
use flynt_core::store::ProjectStore;
use flynt_store::sync::git::{FileHistoryEntry, FileSnapshot, GitSync};
use std::time::Duration;

#[derive(Clone, PartialEq)]
enum EditMode {
    Live,
    Source,
    Diagram,
}

#[derive(Clone, PartialEq)]
#[allow(dead_code)] // Dirty is set via JS DOM manipulation, not Rust
enum SaveState {
    Clean,
    Dirty,
    Saved,
}

#[derive(Clone, Copy, PartialEq)]
enum InspectorTab {
    Links,
    Outline,
    Properties,
}

#[derive(Clone, PartialEq, Debug)]
struct NoteHeading {
    level: usize,
    title: String,
    anchor: String,
    line: usize,
}

#[derive(Clone, PartialEq, Debug, Default)]
struct LinkContext {
    backlinks: Vec<DocumentMeta>,
    outgoing: Vec<OutgoingLinkContext>,
    aliases: Vec<String>,
    resolved_count: usize,
    missing_count: usize,
}

#[derive(Clone, PartialEq, Debug)]
struct OutgoingLinkContext {
    target: String,
    display: Option<String>,
    anchor: Option<String>,
    resolved: Option<DocumentMeta>,
    resolved_artifact: Option<String>,
    count: usize,
}

#[derive(Clone, PartialEq, Debug)]
struct HistoryPanelState {
    entries: Vec<FileHistoryEntry>,
    error: Option<String>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
struct PublishPreviewState {
    output_path: std::path::PathBuf,
    exported: usize,
    skipped_private: usize,
    errors: Vec<String>,
}

#[derive(Clone, Debug, PartialEq)]
struct HoverPreviewState {
    preview: NotePreview,
    x: f64,
    y: f64,
}

#[derive(Clone, PartialEq, Eq, Debug)]
enum PublicationEdit {
    Enabled(bool),
    Visibility(PublicationVisibility),
    Slug(String),
    Collections(String),
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum HistoryDiffKind {
    Context,
    Added,
    Removed,
}

#[derive(Clone, PartialEq, Eq, Debug)]
struct HistoryDiffLine {
    kind: HistoryDiffKind,
    old_line: Option<usize>,
    new_line: Option<usize>,
    text: String,
}

fn build_line_diff(old: &str, new: &str) -> Vec<HistoryDiffLine> {
    let old_lines = old.lines().collect::<Vec<_>>();
    let new_lines = new.lines().collect::<Vec<_>>();
    let mut lcs = vec![vec![0usize; new_lines.len() + 1]; old_lines.len() + 1];

    for i in (0..old_lines.len()).rev() {
        for j in (0..new_lines.len()).rev() {
            lcs[i][j] = if old_lines[i] == new_lines[j] {
                lcs[i + 1][j + 1] + 1
            } else {
                lcs[i + 1][j].max(lcs[i][j + 1])
            };
        }
    }

    let mut out = Vec::new();
    let mut i = 0usize;
    let mut j = 0usize;
    while i < old_lines.len() || j < new_lines.len() {
        if i < old_lines.len() && j < new_lines.len() && old_lines[i] == new_lines[j] {
            out.push(HistoryDiffLine {
                kind: HistoryDiffKind::Context,
                old_line: Some(i + 1),
                new_line: Some(j + 1),
                text: old_lines[i].to_string(),
            });
            i += 1;
            j += 1;
        } else if j < new_lines.len() && (i == old_lines.len() || lcs[i][j + 1] >= lcs[i + 1][j]) {
            out.push(HistoryDiffLine {
                kind: HistoryDiffKind::Added,
                old_line: None,
                new_line: Some(j + 1),
                text: new_lines[j].to_string(),
            });
            j += 1;
        } else if i < old_lines.len() {
            out.push(HistoryDiffLine {
                kind: HistoryDiffKind::Removed,
                old_line: Some(i + 1),
                new_line: None,
                text: old_lines[i].to_string(),
            });
            i += 1;
        }
    }

    out
}

fn build_link_context(
    backlinks: Vec<DocumentMeta>,
    body: &str,
    frontmatter: &Frontmatter,
    mut resolve: impl FnMut(&str) -> Option<DocumentMeta>,
    mut resolve_artifact: impl FnMut(&str) -> Option<String>,
) -> LinkContext {
    let (_, _, links) = parse_document_source(body);
    let mut outgoing = Vec::<OutgoingLinkContext>::new();

    for link in links {
        if let Some(existing) = outgoing.iter_mut().find(|existing| {
            existing.target.eq_ignore_ascii_case(&link.target)
                && existing.anchor == link.anchor
                && existing.display == link.display
        }) {
            existing.count += 1;
            continue;
        }

        let resolved = resolve(&link.target);
        let resolved_artifact = resolved
            .is_none()
            .then(|| resolve_artifact(&link.target))
            .flatten();
        outgoing.push(OutgoingLinkContext {
            target: link.target,
            display: link.display,
            anchor: link.anchor,
            resolved,
            resolved_artifact,
            count: 1,
        });
    }

    let resolved_count = outgoing
        .iter()
        .filter(|link| link.resolved.is_some() || link.resolved_artifact.is_some())
        .count();
    let missing_count = outgoing.len().saturating_sub(resolved_count);

    LinkContext {
        backlinks,
        outgoing,
        aliases: frontmatter.aliases.clone(),
        resolved_count,
        missing_count,
    }
}

fn extract_headings(markdown: &str) -> Vec<NoteHeading> {
    let mut headings = Vec::new();
    let mut seen: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    let mut in_fence = false;

    for (idx, line) in markdown.lines().enumerate() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence || !trimmed.starts_with('#') {
            continue;
        }

        let marker_len = trimmed.chars().take_while(|c| *c == '#').count();
        if marker_len == 0 || marker_len > 6 {
            continue;
        }
        let after = &trimmed[marker_len..];
        if !after.starts_with(' ') {
            continue;
        }
        let title = after.trim().trim_end_matches('#').trim().to_string();
        if title.is_empty() {
            continue;
        }

        let base = heading_anchor(&title);
        let count = seen.entry(base.clone()).or_insert(0);
        let anchor = if *count == 0 {
            base
        } else {
            format!("{base}-{}", *count + 1)
        };
        *count += 1;
        headings.push(NoteHeading {
            level: marker_len,
            title,
            anchor,
            line: idx + 1,
        });
    }

    headings
}

fn heading_anchor(title: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in title.chars().flat_map(|c| c.to_lowercase()) {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_dash = false;
        } else if ch.is_whitespace() || ch == '-' {
            if !last_dash && !out.is_empty() {
                out.push('-');
                last_dash = true;
            }
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        "section".into()
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_toml_and_yaml_frontmatter_without_accepting_unterminated_blocks() {
        assert_eq!(
            content_without_frontmatter("+++\ntitle = \"TOML\"\n+++\n\nBody"),
            "\nBody"
        );
        assert_eq!(
            content_without_frontmatter("---\r\ntitle: YAML\r\n---\r\n\r\nBody"),
            "\r\nBody"
        );
        let malformed = "---\ntitle: Broken\nBody";
        assert_eq!(content_without_frontmatter(malformed), malformed);
    }

    #[test]
    fn renders_github_admonitions_as_semantic_callouts() {
        let html = render_html("> [!TIP]\n> A failed check returns work to Doing.");
        assert!(html.contains("class=\"admonition admonition-tip\""));
        assert!(html.contains("class=\"admonition-title\">Tip</div>"));
        assert!(html.contains("A failed check returns work to Doing."));
        assert!(!html.contains("[!TIP]"));

        let html = render_html("> [!WARNING]\n> Publishing stays blocked.");
        assert!(html.contains("class=\"admonition admonition-warning\""));
        assert!(html.contains("class=\"admonition-title\">Warning</div>"));
        assert!(!html.contains("project://localhost/%21WARNING"));
    }

    #[test]
    fn renders_admonitions_case_insensitively() {
        let html = render_html("> [!note]\n> Lowercase kind marker.");
        assert!(html.contains("class=\"admonition admonition-note\""));
        assert!(html.contains("class=\"admonition-title\">Note</div>"));
        assert!(!html.contains("[!note]"));

        let html = render_html("> [!Note]\n> Title-case kind marker.");
        assert!(html.contains("class=\"admonition admonition-note\""));
        assert!(html.contains("class=\"admonition-title\">Note</div>"));
    }

    #[test]
    fn ordinary_blockquotes_remain_blockquotes() {
        let html = render_html("> Ordinary quoted text.");
        assert!(html.contains("<blockquote>"));
        assert!(!html.contains("class=\"admonition"));
    }

    #[test]
    fn extracts_headings_skipping_fenced_code() {
        let headings = extract_headings(
            r#"# Alpha

```md
## Ignored
```

## Beta!
### Beta!
"#,
        );

        assert_eq!(
            headings,
            vec![
                NoteHeading {
                    level: 1,
                    title: "Alpha".into(),
                    anchor: "alpha".into(),
                    line: 1,
                },
                NoteHeading {
                    level: 2,
                    title: "Beta!".into(),
                    anchor: "beta".into(),
                    line: 7,
                },
                NoteHeading {
                    level: 3,
                    title: "Beta!".into(),
                    anchor: "beta-2".into(),
                    line: 8,
                },
            ]
        );
    }

    #[test]
    fn heading_anchor_normalizes_punctuation_and_whitespace() {
        assert_eq!(
            heading_anchor(" v1.1 Failover & Redundancy "),
            "v11-failover-redundancy"
        );
        assert_eq!(heading_anchor("!!!"), "section");
    }

    #[test]
    fn link_context_preserves_aliases_and_marks_missing_links() {
        let frontmatter = Frontmatter {
            aliases: vec!["Alpha Prime".into()],
            ..Frontmatter::default()
        };
        let resolved = DocumentMeta {
            id: flynt_core::models::DocumentId(uuid::Uuid::new_v4()),
            path: "beta.md".into(),
            title: "Beta".into(),
            tags: vec![],
            metadata: Default::default(),
            entity_kind: None,
            updated_at: chrono::Utc::now(),
        };

        let context = build_link_context(
            vec![],
            "See [[beta|Beta Display]], [[missing]], and [[beta|Beta Display]].",
            &frontmatter,
            |target| {
                target
                    .eq_ignore_ascii_case("beta")
                    .then(|| resolved.clone())
            },
            |_| None,
        );

        assert_eq!(context.aliases, vec!["Alpha Prime"]);
        assert_eq!(context.resolved_count, 1);
        assert_eq!(context.missing_count, 1);
        assert_eq!(context.outgoing.len(), 2);
        assert_eq!(context.outgoing[0].count, 2);
        assert!(context.outgoing[0].resolved.is_some());
        assert!(context.outgoing[1].resolved.is_none());
    }

    #[test]
    fn line_diff_marks_added_removed_and_context_lines() {
        let diff = build_line_diff(
            "alpha\nbeta\ngamma\n",
            "alpha\nbeta changed\ngamma\ndelta\n",
        );

        assert_eq!(
            diff.iter().map(|line| line.kind).collect::<Vec<_>>(),
            vec![
                HistoryDiffKind::Context,
                HistoryDiffKind::Added,
                HistoryDiffKind::Removed,
                HistoryDiffKind::Context,
                HistoryDiffKind::Added,
            ]
        );
        assert_eq!(diff[0].old_line, Some(1));
        assert_eq!(diff[0].new_line, Some(1));
        assert_eq!(diff[1].old_line, None);
        assert_eq!(diff[1].new_line, Some(2));
        assert_eq!(diff[2].old_line, Some(2));
        assert_eq!(diff[2].new_line, None);
    }
}

fn is_d2_path(path: &std::path::Path) -> bool {
    path.extension().is_some_and(|ext| ext == "d2")
}

fn content_without_frontmatter(content: &str) -> &str {
    let (fence, rest) = if let Some(rest) = content
        .strip_prefix("+++\n")
        .or_else(|| content.strip_prefix("+++\r\n"))
    {
        ("+++", rest)
    } else if let Some(rest) = content
        .strip_prefix("---\n")
        .or_else(|| content.strip_prefix("---\r\n"))
    {
        ("---", rest)
    } else {
        return content;
    };
    let mut consumed = 0usize;
    for line in rest.split_inclusive('\n') {
        consumed += line.len();
        if line.trim_end_matches(['\r', '\n']).trim() == fence {
            return &rest[consumed..];
        }
    }
    content
}

fn d2_embed_path(content: &str) -> Option<String> {
    let trimmed = content_without_frontmatter(content).trim();
    if !trimmed.starts_with("![[") || !trimmed.ends_with("]]") {
        return None;
    }
    let inner = &trimmed[3..trimmed.len() - 2];
    let file_ref = inner.split('|').next().unwrap_or(inner).trim();
    file_ref.ends_with(".d2").then(|| file_ref.to_string())
}

fn resolve_d2_path(
    root: &std::path::Path,
    wrapper_path: &std::path::Path,
    content: &str,
) -> std::path::PathBuf {
    if is_d2_path(wrapper_path) {
        return root.join(wrapper_path);
    }
    if let Some(embed) = d2_embed_path(content) {
        let wrapper_dir = wrapper_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new(""));
        let candidates = [
            root.join(wrapper_dir).join(&embed),
            root.join(&embed),
            root.join("diagrams").join(&embed),
            root.join("drawings").join(&embed),
        ];
        return candidates
            .iter()
            .find(|p| p.exists())
            .cloned()
            .unwrap_or_else(|| root.join(wrapper_dir).join(embed));
    }
    root.join(wrapper_path)
}

fn render_html(content: &str) -> String {
    render_html_with_store(content, None, None)
}

fn render_html_with_store(
    content: &str,
    store: Option<&dyn flynt_core::store::ProjectStore>,
    project_root: Option<&std::path::Path>,
) -> String {
    let mut opts = Options::default();
    opts.extension.table = true;
    opts.extension.strikethrough = true;
    opts.extension.tasklist = true;
    opts.extension.autolink = true;
    opts.extension.footnotes = true;
    opts.extension.wikilinks_title_after_pipe = true;
    opts.render.unsafe_ = true;
    let mut html = postprocess_html(markdown_to_html(&preprocess(content), &opts));

    // Execute inline query blocks: <pre><code class="language-query">...</code></pre>
    if let Some(store) = store {
        while let Some(start) = html.find("<code class=\"language-query\">") {
            let code_start = start + "<code class=\"language-query\">".len();
            let Some(code_end) = html[code_start..].find("</code>") else {
                break;
            };
            let code_end = code_start + code_end;

            // Find the wrapping <pre>
            let pre_start = html[..start].rfind("<pre>").unwrap_or(start);
            let pre_end = html[code_end..]
                .find("</pre>")
                .map(|p| code_end + p + 6)
                .unwrap_or(code_end + 7);

            let query_source = html_unescape(&html[code_start..code_end]);
            let result = match flynt_core::query::execute_query(&query_source, store) {
                Ok(rendered) => format!("<div class=\"query-result\">{rendered}</div>"),
                Err(e) => format!(
                    "<div class=\"query-error\">This query could not run: {e}<br><small>Syntax: <code>TABLE title, tags FROM \"\" WHERE tags = \"#tag\" SORT title</code></small></div>"
                ),
            };

            html = format!("{}{}{}", &html[..pre_start], result, &html[pre_end..]);
        }
    }

    // Embed Excalidraw drawings: ![[file.excalidraw]] → inline SVG
    // Also handles image embeds: ![[image.png]] → <img src="project://...">
    if let Some(root) = project_root {
        // Pattern: ![[something.excalidraw]] (may appear as text or inside <p> tags)
        while let Some(start) = html.find("![[") {
            let Some(end) = html[start..].find("]]") else {
                break;
            };
            let end = start + end;
            let ref_name = &html[start + 3..end];

            if ref_name.contains(".excalidraw") {
                // Parse optional width: ![[drawing.excalidraw|400]]
                let (file_ref, width) = if let Some(pipe) = ref_name.find('|') {
                    (&ref_name[..pipe], Some(&ref_name[pipe + 1..]))
                } else {
                    (ref_name, None)
                };

                // Search for the .excalidraw file in common locations
                let candidates = [root.join(file_ref), root.join("drawings").join(file_ref)];
                let excalidraw_path = candidates
                    .iter()
                    .find(|p| p.exists())
                    .cloned()
                    .unwrap_or_else(|| root.join(file_ref));
                let svg_path = excalidraw_path.with_extension("svg");
                let style = width
                    .map(|w| format!(" style=\"max-width:{w}px\""))
                    .unwrap_or_default();
                let escaped_ref = file_ref.replace('"', "&quot;");

                let replacement = if svg_path.exists() {
                    match std::fs::read_to_string(&svg_path) {
                        Ok(svg) => format!(
                            "<div class=\"excalidraw-embed\" data-drawing=\"{escaped_ref}\"{style}>{svg}</div>"
                        ),
                        Err(_) => format!(
                            "<div class=\"excalidraw-embed-placeholder\">[Drawing: {file_ref}]</div>"
                        ),
                    }
                } else if excalidraw_path.exists() {
                    format!(
                        "<div class=\"excalidraw-embed-placeholder\" data-drawing=\"{escaped_ref}\">[Drawing: {file_ref} — open to render]</div>"
                    )
                } else {
                    format!(
                        "<span class=\"broken-embed\">Embedded file not found: {file_ref}</span>"
                    )
                };

                html = format!("{}{}{}", &html[..start], replacement, &html[end + 2..]);
            } else if ref_name.contains(".d2") {
                // D2 diagram embed — same pattern as Excalidraw: look for .svg sidecar
                let (file_ref, width) = if let Some(pipe) = ref_name.find('|') {
                    (&ref_name[..pipe], Some(&ref_name[pipe + 1..]))
                } else {
                    (ref_name, None)
                };

                let candidates = [
                    root.join(file_ref),
                    root.join("diagrams").join(file_ref),
                    root.join("drawings").join(file_ref),
                    root.join("boards").join(file_ref),
                ];
                let d2_path = candidates
                    .iter()
                    .find(|p| p.exists())
                    .cloned()
                    .unwrap_or_else(|| root.join(file_ref));
                let svg_path = d2_path.with_extension("svg");
                let style = width
                    .map(|w| format!(" style=\"max-width:{w}px\""))
                    .unwrap_or_default();

                let replacement = if svg_path.exists() {
                    match std::fs::read_to_string(&svg_path) {
                        Ok(svg) => format!("<div class=\"d2-embed\"{style}>{svg}</div>"),
                        Err(_) => format!(
                            "<div class=\"d2-embed-placeholder\">[Diagram: {file_ref}]</div>"
                        ),
                    }
                } else if d2_path.exists() {
                    format!(
                        "<div class=\"d2-embed-placeholder\">[Diagram: {file_ref} — rendering not available]</div>"
                    )
                } else {
                    format!(
                        "<span class=\"broken-embed\">Diagram file not found: {file_ref}</span>"
                    )
                };

                html = format!("{}{}{}", &html[..start], replacement, &html[end + 2..]);
            } else if ref_name.ends_with(".png")
                || ref_name.ends_with(".jpg")
                || ref_name.ends_with(".jpeg")
                || ref_name.ends_with(".gif")
                || ref_name.ends_with(".svg")
                || ref_name.ends_with(".webp")
            {
                // Image embed — resolve path, searching common locations
                let image_candidates = [
                    ref_name.to_string(),
                    format!("assets/{ref_name}"),
                    format!("images/{ref_name}"),
                    format!("drawings/{ref_name}"),
                ];
                let resolved = image_candidates
                    .iter()
                    .find(|p| root.join(p).exists())
                    .cloned()
                    .unwrap_or_else(|| ref_name.to_string());
                let encoded = resolved.replace(' ', "%20");
                let replacement = format!(
                    "<img class=\"embedded-image\" src=\"project://localhost/{encoded}\" alt=\"{ref_name}\" />"
                );
                html = format!("{}{}{}", &html[..start], replacement, &html[end + 2..]);
            } else {
                break; // not an embed we handle — avoid infinite loop
            }
        }
    }

    // Replace bare external URLs with smart badges
    // Match <a href="https://...">https://...</a> (autolinked URLs where text == href)
    let mut out = String::with_capacity(html.len());
    let mut search_from = 0;
    while let Some(start) = html[search_from..].find("<a href=\"http") {
        let abs_start = search_from + start;
        if let Some(close) = html[abs_start..].find("</a>") {
            let tag_end = abs_start + close + 4;
            let tag = &html[abs_start..tag_end];
            // Extract href
            if let (Some(href_start), Some(href_end)) = (tag.find("href=\""), tag.find("\">")) {
                let href = &tag[href_start + 6..href_end];
                // Extract link text
                let text_start = href_end + 2;
                let text_end = tag.len() - 4; // before </a>
                let text = &tag[text_start..text_end];
                // Only replace if link text IS the URL (autolinked) or starts with http
                if text.starts_with("http") || text == href {
                    let ext_ref = flynt_core::external_ref::parse_ref(href);
                    if ext_ref.provider != flynt_core::external_ref::Provider::Generic {
                        let badge = flynt_core::external_ref::render_html(&ext_ref);
                        out.push_str(&html[search_from..abs_start]);
                        out.push_str(&badge);
                        search_from = tag_end;
                        continue;
                    }
                }
            }
            out.push_str(&html[search_from..tag_end]);
            search_from = tag_end;
        } else {
            break;
        }
    }
    out.push_str(&html[search_from..]);
    out
}

fn html_unescape(s: &str) -> String {
    s.replace("&amp;", "&")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&apos;", "'")
        .replace("&#x27;", "'")
        .replace("&nbsp;", " ")
        .replace("&#34;", "\"")
}

fn postprocess_html(html: String) -> String {
    let pattern = "href=\"flynt-note://";
    let mut result = String::with_capacity(html.len());
    let mut rest = html.as_str();
    while let Some(idx) = rest.find(pattern) {
        result.push_str(&rest[..idx]);
        let after = &rest[idx + pattern.len()..];
        if let Some(end) = after.find('"') {
            let slug = &after[..end];
            result.push_str("href=\"#\" data-flynt-note=\"");
            result.push_str(slug);
            result.push('"');
            rest = &after[end + 1..];
        } else {
            result.push_str(&rest[idx..]);
            break;
        }
    }
    result.push_str(rest);
    render_admonitions(result)
}

fn render_admonitions(mut html: String) -> String {
    const KINDS: [(&str, &str); 6] = [
        ("NOTE", "Note"),
        ("TIP", "Tip"),
        ("IMPORTANT", "Important"),
        ("WARNING", "Warning"),
        ("CAUTION", "Caution"),
        ("DANGER", "Danger"),
    ];

    for (marker, title) in KINDS {
        // Comrak lowercases nothing in the source text it emits, so a
        // `[!note]` or `[!Note]` marker survives verbatim into the HTML.
        // Match case-insensitively (mirroring the CM6 live-preview regex's
        // `/i` flag) by searching an ASCII-lowercased copy of the HTML —
        // byte offsets stay aligned since only ASCII letters change case.
        let marker_lower = marker.to_ascii_lowercase();
        let needles = [
            format!("<blockquote>\n<p>[!{marker_lower}]"),
            format!(
                "<blockquote>\n<p><a href=\"project://localhost/%21{marker_lower}\">!{marker_lower}</a>"
            ),
        ];
        let replacement = format!(
            "<aside class=\"admonition admonition-{}\" role=\"note\"><div class=\"admonition-title\">{title}</div><div class=\"admonition-body\"><p>",
            marker_lower
        );
        loop {
            let haystack_lower = html.to_ascii_lowercase();
            let Some((start, needle_len)) = needles
                .iter()
                .filter_map(|needle| {
                    haystack_lower
                        .find(needle.as_str())
                        .map(|start| (start, needle.len()))
                })
                .min_by_key(|(start, _)| *start)
            else {
                break;
            };
            let body_start = start + needle_len;
            let Some(relative_end) = haystack_lower[body_start..].find("</blockquote>") else {
                break;
            };
            let end = body_start + relative_end;
            html.replace_range(
                start..end + "</blockquote>".len(),
                &format!(
                    "{}{}{}",
                    replacement,
                    &html[body_start..end],
                    "</div></aside>"
                ),
            );
        }
    }
    html
}

fn preprocess(src: &str) -> String {
    let mut out = String::with_capacity(src.len() + 64);
    let mut chars = src.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '!' && chars.peek() == Some(&'[') {
            chars.next();
            if chars.peek() == Some(&'[') {
                chars.next();
                let inner: String = chars.by_ref().take_while(|&ch| ch != ']').collect();
                if chars.peek() == Some(&']') {
                    chars.next();
                }
                let ext = std::path::Path::new(&inner)
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("");
                if matches!(ext, "png" | "jpg" | "jpeg" | "gif" | "svg" | "webp") {
                    let encoded = inner.replace(' ', "%20");
                    out.push_str(&format!("![{inner}](project://localhost/{encoded})"));
                } else {
                    let encoded = inner.replace(' ', "%20");
                    out.push_str(&format!("[{inner}](project://localhost/{encoded})"));
                }
                continue;
            } else {
                out.push('!');
                out.push('[');
            }
        } else if c == '[' && chars.peek() == Some(&'[') {
            chars.next();
            let inner: String = chars.by_ref().take_while(|&ch| ch != ']').collect();
            if chars.peek() == Some(&']') {
                chars.next();
            }
            let (target, display) = inner
                .split_once('|')
                .map(|(t, d)| (t, d))
                .unwrap_or((&inner, &inner as &str));
            let encoded = target.replace(' ', "%20");
            out.push_str(&format!("[{display}](flynt-note://{encoded})"));
            continue;
        }
        out.push(c);
    }
    out
}

// ── CM6 init JS ─────────────────────────────────────────────────────────────

fn build_artifact_link_index(root: &std::path::Path) -> std::collections::HashMap<String, String> {
    let mut out = std::collections::HashMap::new();
    let mut artifacts = Vec::new();
    artifacts.extend(flynt_core::visual_artifacts::discover_excalidraw_artifacts(
        root,
    ));
    artifacts.extend(flynt_core::visual_artifacts::discover_design_board_artifacts(root));
    artifacts.extend(flynt_core::visual_artifacts::discover_flow_artifacts(root));

    for artifact in artifacts {
        let kind = match artifact.kind {
            flynt_core::visual_artifacts::VisualArtifactKind::ExcalidrawDrawing => "drawing",
            flynt_core::visual_artifacts::VisualArtifactKind::DesignBoard => "board",
            flynt_core::visual_artifacts::VisualArtifactKind::Flow => "flow",
            flynt_core::visual_artifacts::VisualArtifactKind::D2Diagram => "artifact",
        };
        let source = artifact.source_path.to_string_lossy().to_string();
        insert_artifact_link_alias(&mut out, &artifact.title, kind);
        insert_artifact_link_alias(&mut out, &source, kind);
        if let Some(name) = artifact
            .source_path
            .file_name()
            .and_then(|name| name.to_str())
        {
            insert_artifact_link_alias(&mut out, name, kind);
        }
        if let Some(wrapper) = artifact.wrapper_path {
            let wrapper = wrapper.to_string_lossy().to_string();
            insert_artifact_link_alias(&mut out, &wrapper, kind);
        }
    }
    out
}

fn insert_artifact_link_alias(
    out: &mut std::collections::HashMap<String, String>,
    value: &str,
    kind: &str,
) {
    out.insert(value.to_string(), kind.to_string());
    out.insert(value.to_lowercase(), kind.to_string());
    out.insert(value.to_lowercase().replace(' ', "-"), kind.to_string());
}

fn resolve_artifact_link(
    index: &std::collections::HashMap<String, String>,
    target: &str,
) -> Option<String> {
    let cleaned = target.trim();
    index
        .get(cleaned)
        .or_else(|| index.get(&cleaned.to_lowercase()))
        .or_else(|| index.get(&cleaned.to_lowercase().replace(' ', "-")))
        .cloned()
}

fn format_doc_timestamp(ts: chrono::DateTime<chrono::Utc>) -> String {
    ts.with_timezone(&chrono::Local)
        .format("%b %-d, %Y")
        .to_string()
}

fn cm6_init_js(doc_id: &DocumentId, content: &str, embed_index_json: &str) -> String {
    let doc_id_json =
        serde_json::to_string(&doc_id.0.to_string()).unwrap_or_else(|_| "null".into());
    let escaped = serde_json::to_string(content).unwrap_or_else(|_| "\"\"".into());
    let embed_index = if embed_index_json.trim().is_empty() {
        "{}"
    } else {
        embed_index_json
    };
    format!(
        r#"
(function() {{
    function _initCM() {{
    const container = document.getElementById('flynt-cm-editor');
    window._flyntActiveDocId = {doc_id_json};
    window._flyntEmbedIndex = {embed_index};
    if (!window.FlyntEmbedResolver) {{
        window.FlyntEmbedResolver = {{
            resolve(ref) {{
                const cleaned = String(ref || '').trim();
                const key = cleaned.toLowerCase().replace(/\s+/g, '-');
                const index = window._flyntEmbedIndex || {{}};
                return index[cleaned] || index[key] || {{ status: 'missing', ref: cleaned, kind: 'unknown', surface: 'unknown', icon: '?', label: cleaned }};
            }},
            imageUrls(resolution) {{
                const ref = resolution.canonicalPath || resolution.ref;
                const encoded = encodeURIComponent(ref).replace(/%2F/g, '/');
                return ['project://localhost/' + encoded, 'project://localhost/assets/' + encoded, 'project://localhost/images/' + encoded, 'project://localhost/drawings/' + encoded];
            }},
            open(resolution) {{ window._flyntNotify('editor.embed.open', JSON.stringify(resolution)); }}
        }};
    }}
    if (!container) {{ setTimeout(_initCM, 16); return; }}

    console.time('cm6-total');
    // Fast path: if CM6 already exists AND its DOM is still attached
    // to the current container, swap the document content in place.
    //
    // Why the attachment check: when the operator toggles Source then
    // back to Live, Dioxus unmounts the editor div and mounts a fresh
    // one (same id, different DOM node). `window._flyntCM` still
    // references the OLD editor whose root DOM is now detached.
    // Dispatching content to that editor draws nothing — the new div
    // stays blank until a re-init. So if attachment is broken, fall
    // through to the full init path which rebuilds the editor under
    // the fresh container.
    if (window._flyntCM) {{
        const cm = window._flyntCM;
        const stillAttached = cm.dom && container.contains(cm.dom);
        if (stillAttached) {{
            console.time('cm6-swap');
            const newContent = {escaped};
            if (window.FlyntEditor) {{
                window.FlyntEditor.setDocument({{ content: newContent }}, {{ force: true }});
                const state = window.FlyntEditor.getEditorState && window.FlyntEditor.getEditorState();
                if (state) window.FlyntEditor.restoreEditorState({{ ...state, scrollTop: 0, scrollLeft: 0 }});
            }} else {{
                cm.dispatch({{ changes: {{ from: 0, to: cm.state.doc.length, insert: newContent }} }});
                cm.scrollDOM.scrollTop = 0;
            }}
            console.timeEnd('cm6-swap');
            console.timeEnd('cm6-total');
            return;
        }} else {{
            if (window.FlyntEditor && typeof window.FlyntEditor.unmount === 'function') {{
                window.FlyntEditor.unmount();
            }} else {{
                try {{ cm.destroy(); }} catch(e) {{}}
                window._flyntCM = null;
            }}
            // fall through to full init
        }}
    }}
    console.time('cm6-init');
    container.innerHTML = '';

    const {{
        EditorView, Decoration, WidgetType, keymap, drawSelection, highlightActiveLine,
        highlightSpecialChars,
        EditorState,
        defaultKeymap, history, historyKeymap, indentWithTab,
        markdown, markdownLanguage, GFM,
        languages,
        syntaxHighlighting, defaultHighlightStyle, bracketMatching,
        oneDark,
        closeBrackets,
        searchKeymap, highlightSelectionMatches,
        HighlightStyle, tags,
        createLivePreview, createBlockRender, createFrontmatterHider,
    }} = CM;

    const livePreview = createLivePreview();

    class TableWidget extends WidgetType {{
        constructor(html) {{ super(); this._html = html; }}
        toDOM() {{
            const d = document.createElement('div');
            d.className = 'cm-table-widget';
            d.innerHTML = this._html;
            return d;
        }}
        ignoreEvent() {{ return false; }}
        eq(o) {{ return this._html === o._html; }}
    }}

    // Inline-safe counterpart to TableWidget — its <div> wrapper forces a
    // block-level break, which is wrong for a marker meant to sit at the
    // end of an existing line (it pushed the marker onto its own row).
    class InlineWidget extends WidgetType {{
        constructor(html) {{ super(); this._html = html; }}
        toDOM() {{
            const s = document.createElement('span');
            s.innerHTML = this._html;
            return s;
        }}
        ignoreEvent() {{ return false; }}
        eq(o) {{ return this._html === o._html; }}
    }}

    // Minimal inline markdown -> HTML for static widget bodies (admonitions,
    // table cells) that are rendered once as a plain HTML string rather than
    // through CodeMirror decorations. Code spans are extracted first so their
    // content is never reinterpreted as bold/italic/link syntax, then
    // restored verbatim after everything else runs.
    function renderInlineMd(raw) {{
        const escapeHtml = value => value
            .replace(/&/g, '&amp;')
            .replace(/</g, '&lt;')
            .replace(/>/g, '&gt;')
            .replace(/"/g, '&quot;');
        const mark = String.fromCharCode(1);
        const markRe = new RegExp(mark + '(\\d+)' + mark, 'g');
        const codeSpans = [];
        let text = raw.replace(/`([^`]+?)`/g, (_, code) => {{
            codeSpans.push(escapeHtml(code));
            return mark + (codeSpans.length - 1) + mark;
        }});
        text = escapeHtml(text);
        text = text.replace(/\[\[([^\]|]+)(?:\|([^\]]+))?\]\]/g, (_, target, display) =>
            '<span class="cm-wikilink">' + (display || target) + '</span>');
        text = text.replace(/\*\*([^*]+?)\*\*/g, '<strong>$1</strong>');
        text = text.replace(/__([^_]+?)__/g, '<strong>$1</strong>');
        text = text.replace(/~~([^~]+?)~~/g, '<del>$1</del>');
        text = text.replace(/\*([^*]+?)\*/g, '<em>$1</em>');
        text = text.replace(/_([^_]+?)_/g, '<em>$1</em>');
        text = text.replace(markRe, (_, idx) => '<code>' + codeSpans[Number(idx)] + '</code>');
        return text;
    }}

    const flyntTheme = window.FlyntEditorCompat.themeExtension(EditorView);

    const flyntHighlight = HighlightStyle.define([
        {{ tag: tags.heading1, fontSize: '1.8em', fontWeight: '700', color: 'var(--prose-heading, #f1f5f9)' }},
        {{ tag: tags.heading2, fontSize: '1.5em', fontWeight: '600', color: 'var(--prose-heading, #f1f5f9)' }},
        {{ tag: tags.heading3, fontSize: '1.25em', fontWeight: '600', color: 'var(--prose-heading, #f1f5f9)' }},
        {{ tag: tags.heading4, fontSize: '1.1em', fontWeight: '600', color: 'var(--prose-heading, #f1f5f9)' }},
        {{ tag: tags.heading5, fontSize: '1.05em', fontWeight: '600', color: 'var(--prose-heading, #f1f5f9)' }},
        {{ tag: tags.heading6, fontSize: '1em', fontWeight: '600', color: 'var(--prose-heading, #f1f5f9)' }},
        {{ tag: tags.processingInstruction, color: 'var(--muted-foreground, #475569)', fontSize: '0.85em' }},
        {{ tag: tags.strong, fontWeight: '700', color: 'var(--prose-heading, #f1f5f9)' }},
        {{ tag: tags.emphasis, fontStyle: 'italic' }},
        {{ tag: tags.strikethrough, textDecoration: 'line-through', color: 'var(--muted-foreground, #64748b)' }},
        {{ tag: tags.url, color: 'var(--prose-link, #4cc9f0)' }},
        {{ tag: tags.link, color: 'var(--prose-link, #4cc9f0)', textDecoration: 'underline' }},
        {{ tag: tags.monospace, fontFamily: 'var(--font-mono)', color: 'var(--prose-code-fg, #e2e8f0)', backgroundColor: 'var(--prose-code-bg, rgba(30,41,59,0.7))', borderRadius: '3px', padding: '1px 4px' }},
        {{ tag: tags.list, color: 'var(--ring, #2ab4c8)' }},
        {{ tag: tags.quote, color: 'var(--muted-foreground, #94a3b8)', fontStyle: 'italic' }},
        {{ tag: tags.meta, color: 'var(--muted-foreground, #475569)' }},
        {{ tag: tags.content, color: 'var(--prose-body, #d7e0ea)' }},
    ]);

    // ── Live preview: hide markdown punctuation on non-active lines ──
    const hideMarkupPlugin = EditorView.decorations.compute(['doc', 'selection'], (state) => {{ try {{
        const decs = [];
        const doc = state.doc;
        const sel = state.selection.main;
        const activeLine = doc.lineAt(sel.head).number;

        // Performance: only hide markup on small documents
        if (doc.lines > 150) return Decoration.none;

        // Hide TOML frontmatter (+++ ... +++)
        let fmStart = -1, fmEnd = -1;
        if (doc.lines >= 1 && doc.line(1).text.trim() === '+++') {{
            fmStart = 1;
            for (let j = 2; j <= doc.lines; j++) {{
                if (doc.line(j).text.trim() === '+++') {{ fmEnd = j; break; }}
            }}
        }}
        if (fmStart > 0 && fmEnd > 0) {{
            for (let fl = fmStart; fl <= fmEnd; fl++) {{
                const fline = doc.line(fl);
                if (fline.length > 0) {{
                    decs.push(Decoration.replace({{}}).range(fline.from, fline.to));
                }}
            }}
        }}

        for (let i = 1; i <= doc.lines; i++) {{
            if (fmStart > 0 && fmEnd > 0 && i >= fmStart && i <= fmEnd) continue; // skip frontmatter lines
            const line = doc.line(i);
            const text = line.text;

            // Horizontal rule: --- or *** or ___ → styled line
            if (text.trim() === '---' || text.trim() === '***' || text.trim() === '___') {{
                decs.push(Decoration.replace({{}}).range(line.from, line.to));
                decs.push(Decoration.line({{ class: 'cm-hr-line' }}).range(line.from));
                continue;
            }}

            // Blockquote: hide the '>' marker(s) (plus one following space)
            // and style the whole line as a quote, indented per nesting
            // depth (count of '>' chars) so >> / >>> visibly step further
            // right than a single '>'. Admonitions ([!KIND] quotes) are
            // matched and consumed above, so only ordinary blockquote lines
            // reach here. No continue — inline formatting below still
            // applies to the quoted text.
            const quoteMatch = text.match(/^((?:\s{{0,3}}>)+\s?)/);
            const quoteDepthClass = depth => 'cm-quote-line' + (depth > 1 ? ' cm-quote-line-' + Math.min(depth, 4) : '');
            if (quoteMatch) {{
                const depth = (quoteMatch[1].match(/>/g) || []).length;
                decs.push(Decoration.replace({{}}).range(line.from, line.from + quoteMatch[1].length));
                decs.push(Decoration.line({{ class: quoteDepthClass(depth) }}).range(line.from));
            }} else if (i > 1 && text.trim() !== '') {{
                const prevMatch = doc.line(i - 1).text.match(/^((?:\s{{0,3}}>)+)/);
                if (prevMatch) {{
                    // Lazy continuation: no marker on this line, but the
                    // previous line was a blockquote line and this one isn't
                    // blank — CommonMark treats it as part of the same quote,
                    // at the same nesting depth.
                    const depth = (prevMatch[1].match(/>/g) || []).length;
                    decs.push(Decoration.line({{ class: quoteDepthClass(depth) }}).range(line.from));
                }}
            }}

            // Hide heading markers
            if (text.match(/^#/) && text.indexOf(' ') > 0 && text.indexOf(' ') <= 7) {{
                const spaceIdx = text.indexOf(' ');
                decs.push(Decoration.replace({{}}).range(line.from, line.from + spaceIdx + 1));
                continue;
            }}

            // Backslash escapes: \X hides the backslash and leaves X as
            // literal text — X must not be treated as emphasis/code syntax
            // by the marker loops below, so they consult `escaped`.
            const escaped = new Set();
            text.replace(/\\([!#$%&'()*+,\-./:;<=>?@[\]^_`{{|}}~"\\])/g, (m, ch, offset) => {{
                decs.push(Decoration.replace({{}}).range(line.from + offset, line.from + offset + 1));
                escaped.add(offset + 1);
                return m;
            }});

            // Hard line break: two-or-more trailing spaces, or an odd
            // number of trailing backslashes, forces a break within the
            // same paragraph. Per CommonMark both are removed entirely from
            // rendered output — same as a soft-wrapped line ending, there's
            // nothing left to distinguish it unless we add our own marker.
            {{
                const trailingSpaces = text.match(/ {{2,}}$/);
                const trailingBackslashes = text.match(/\\+$/);
                const hasBackslashBreak = Boolean(
                    trailingBackslashes && trailingBackslashes[0].length % 2 === 1
                );
                if (trailingSpaces || hasBackslashBreak) {{
                    if (hasBackslashBreak) {{
                        decs.push(Decoration.replace({{}}).range(line.to - 1, line.to));
                    }}
                    decs.push(Decoration.widget({{
                        widget: new InlineWidget('<span class="cm-hardbreak" title="Hard line break">↵</span>'),
                        side: 1,
                    }}).range(line.to));
                }}
            }}

            // Hide bold markers: **text** (max 50 iterations per line)
            let idx = 0, safety = 0;
            while ((idx = text.indexOf('**', idx)) !== -1 && safety++ < 50) {{
                if (escaped.has(idx) || escaped.has(idx + 1)) {{ idx += 2; continue; }}
                const end = text.indexOf('**', idx + 2);
                if (end > idx) {{
                    decs.push(Decoration.replace({{}}).range(line.from + idx, line.from + idx + 2));
                    decs.push(Decoration.replace({{}}).range(line.from + end, line.from + end + 2));
                    idx = end + 2;
                }} else break;
            }}

            // Hide wikilink brackets: [[target]] or [[target|display]]
            idx = 0; safety = 0;
            while ((idx = text.indexOf('[[', idx)) !== -1 && safety++ < 50) {{
                const end = text.indexOf(']]', idx + 2);
                if (end > idx) {{
                    const inner = text.substring(idx + 2, end);
                    const pipe = inner.indexOf('|');
                    if (pipe >= 0) {{
                        decs.push(Decoration.replace({{}}).range(line.from + idx, line.from + idx + 2 + pipe + 1));
                        decs.push(Decoration.replace({{}}).range(line.from + end, line.from + end + 2));
                    }} else {{
                        decs.push(Decoration.replace({{}}).range(line.from + idx, line.from + idx + 2));
                        decs.push(Decoration.replace({{}}).range(line.from + end, line.from + end + 2));
                    }}
                    idx = end + 2;
                }} else break;
            }}

            // Hide inline code backticks: `code` and multi-backtick spans
            // like ``code with a literal ` backtick``. Per CommonMark, the
            // closing delimiter must be a run of backticks the SAME LENGTH
            // as the opening run — a shorter or longer run inside is just
            // literal content, not a close. The previous version treated
            // any '`' as a single-backtick delimiter, so an opening ``
            // never matched as a pair and a lone backtick nested inside
            // got misread as the real code span instead.
            idx = 0;
            safety = 0;
            while (idx < text.length && safety++ < 50) {{
                idx = text.indexOf('`', idx);
                if (idx === -1) break;
                let openEnd = idx;
                while (text.charAt(openEnd) === '`') openEnd++;
                const runLen = openEnd - idx;
                let searchFrom = openEnd, closeStart = -1, closeEnd = -1;
                while (searchFrom < text.length) {{
                    const nextTick = text.indexOf('`', searchFrom);
                    if (nextTick === -1) break;
                    let runEnd = nextTick;
                    while (text.charAt(runEnd) === '`') runEnd++;
                    if (runEnd - nextTick === runLen) {{ closeStart = nextTick; closeEnd = runEnd; break; }}
                    searchFrom = runEnd;
                }}
                if (closeStart === -1) {{ idx = openEnd; continue; }}
                decs.push(Decoration.replace({{}}).range(line.from + idx, line.from + openEnd));
                if (closeStart > openEnd) {{
                    decs.push(Decoration.mark({{ class: 'cm-code-mark' }}).range(line.from + openEnd, line.from + closeStart));
                }}
                decs.push(Decoration.replace({{}}).range(line.from + closeStart, line.from + closeEnd));
                idx = closeEnd;
            }}

            // Bold/italic via underscores: _text_ and __text__. Unlike '**',
            // the underscore characters themselves stay visible (not hidden)
            // — they're kept as literal text, just styled bold/italic along
            // with the content they wrap, via an explicit cm-strong-mark/
            // cm-em-mark since the language mode doesn't reliably tag
            // underscore-delimited runs strong/emphasis on its own.
            idx = 0; safety = 0;
            while ((idx = text.indexOf('__', idx)) !== -1 && safety++ < 50) {{
                if (escaped.has(idx) || escaped.has(idx + 1)) {{ idx += 2; continue; }}
                const end = text.indexOf('__', idx + 2);
                if (end > idx) {{
                    decs.push(Decoration.mark({{ class: 'cm-strong-mark' }}).range(line.from + idx, line.from + end + 2));
                    idx = end + 2;
                }} else break;
            }}
            idx = 0;
            while (idx < text.length) {{
                idx = text.indexOf('_', idx);
                if (idx === -1) break;
                if (escaped.has(idx)) {{ idx++; continue; }}
                if (text.charAt(idx - 1) === '_' || text.charAt(idx + 1) === '_') {{ idx++; continue; }} // skip __
                if (idx > 0 && text.charAt(idx - 1).match(/[a-zA-Z0-9]/)) {{ idx++; continue; }} // mid-word
                const end = text.indexOf('_', idx + 1);
                if (end > idx && !(text.charAt(end - 1) === '_' || text.charAt(end + 1) === '_')) {{
                    decs.push(Decoration.mark({{ class: 'cm-em-mark' }}).range(line.from + idx, line.from + end + 1));
                    idx = end + 1;
                }} else {{ idx++; }}
            }}

            // Render GitHub/Obsidian admonitions in Live mode. The previous
            // HTML postprocessor only affected static preview; notes normally
            // use this CodeMirror surface.
            const admonition = text.match(/^\s*>\s*\[!(NOTE|TIP|IMPORTANT|WARNING|CAUTION|DANGER)\]\s*$/i);
            if (admonition) {{
                let end = i;
                while (end < doc.lines && /^\s*>/.test(doc.line(end + 1).text)) end++;
                const from = line.from;
                const to = doc.line(end).to;
                if (sel.head < from || sel.head > to) {{
                    const kind = admonition[1].toLowerCase();
                    const title = kind.charAt(0).toUpperCase() + kind.slice(1);
                    const body = [];
                    for (let row = i + 1; row <= end; row++) {{
                        body.push(doc.line(row).text.replace(/^\s*>\s?/, ''));
                    }}
                    const h = '<aside class="admonition admonition-' + kind + '">' +
                        '<div class="admonition-title">' + title + '</div>' +
                        '<div class="admonition-body"><p>' + renderInlineMd(body.join('\n')) + '</p></div></aside>';
                    decs.push(Decoration.replace({{ widget: new TableWidget(h) }}).range(from, to));
                }}
                i = end;
                continue;
            }}

            // Tables — find full table block and replace with rendered widget
            if (text.indexOf('|') >= 0 && text.trim().charAt(0) === '|') {{
                // Find table extent
                let ts = i, te = i;
                while (ts > 1 && doc.line(ts-1).text.trim().startsWith('|')) ts--;
                while (te < doc.lines && doc.line(te+1).text.trim().startsWith('|')) te++;

                if (i === ts) {{
                    const tFrom = doc.line(ts).from;
                    const tTo = doc.line(te).to;
                    if (sel.head < tFrom || sel.head > tTo) {{
                        // Parse table
                        let rows = [], hasSep = false;
                        for (let r = ts; r <= te; r++) {{
                            const rt = doc.line(r).text.trim();
                            let allSep = true;
                            for (let c = 0; c < rt.length; c++) {{
                                if ('|-: '.indexOf(rt.charAt(c)) < 0) {{ allSep = false; break; }}
                            }}
                            if (allSep) {{ hasSep = true; continue; }}
                            rows.push(rt.split('|').slice(1,-1).map(s => s.trim()));
                        }}
                        if (rows.length > 0) {{
                            let h = '<table class="cm-rendered-table">';
                            rows.forEach((cells, ri) => {{
                                h += '<tr>';
                                const t = ri === 0 ? 'th' : 'td';
                                cells.forEach(c => {{
                                    let v = renderInlineMd(c);
                                    h += '<'+t+'>'+v+'</'+t+'>';
                                }});
                                h += '</tr>';
                            }});
                            h += '</table>';
                            const to = Math.min(tTo + 1, doc.length);
                            decs.push(Decoration.replace({{ widget: new TableWidget(h) }}).range(tFrom, to));
                        }}
                    }}
                }}
                i = te;
                continue;
            }}

            // Hide unordered list markers: "- " or "* " or "+ " at line start
            const trimmed = text.trimStart();
            const indent = text.length - trimmed.length;
            if ((trimmed.startsWith('- ') || trimmed.startsWith('* ') || trimmed.startsWith('+ ')) && trimmed.length > 2) {{
                decs.push(Decoration.replace({{}}).range(line.from + indent, line.from + indent + 2));
            }}

            // Hide strikethrough: ~~text~~
            idx = 0;
            while ((idx = text.indexOf('~~', idx)) !== -1) {{
                const end = text.indexOf('~~', idx + 2);
                if (end > idx) {{
                    decs.push(Decoration.replace({{}}).range(line.from + idx, line.from + idx + 2));
                    decs.push(Decoration.replace({{}}).range(line.from + end, line.from + end + 2));
                    idx = end + 2;
                }} else break;
            }}
        }}

        // Hide frontmatter block (+++...+++)
        let fmState = 0; // 0=before, 1=inside, 2=done
        for (let i = 1; i <= doc.lines && fmState < 2; i++) {{
            const line = doc.line(i);
            if (line.text.trim() === '+++') {{
                if (fmState === 0) {{ fmState = 1; }}
                else {{ fmState = 2; }}
                if (i !== activeLine) {{
                    decs.push(Decoration.replace({{}}).range(line.from, Math.min(line.to + 1, doc.length)));
                }}
            }} else if (fmState === 1 && i !== activeLine) {{
                decs.push(Decoration.replace({{}}).range(line.from, Math.min(line.to + 1, doc.length)));
            }}
        }}

        // CM6 requires decorations sorted by from position
        return Decoration.set(decs, true);
    }} catch(e) {{ window._hideErr = e.message; return Decoration.none; }}
    }});

    // ── Table styling: add CSS classes to table lines ──
    // Combined decoration plugin — single pass over all lines for tables, code blocks, tasks, embeds
    const combinedPlugin = EditorView.decorations.compute(['doc'], (state) => {{
        const decs = [];
        const doc = state.doc;
        let inTable = false, isHeader = true, inCodeBlock = false;

        for (let i = 1; i <= doc.lines; i++) {{
            const line = doc.line(i);
            const text = line.text;
            const trimmed = text.trim();

            // Code blocks
            if (!inCodeBlock && trimmed.startsWith('```')) {{
                inCodeBlock = true;
                const lang = trimmed.slice(3).trim();
                decs.push(Decoration.line({{
                    class: 'cm-codeblock-fence cm-codeblock-first',
                    attributes: {{ 'data-lang': lang }},
                }}).range(line.from));
                continue;
            }} else if (inCodeBlock && trimmed.startsWith('```')) {{
                inCodeBlock = false;
                decs.push(Decoration.line({{ class: 'cm-codeblock-fence cm-codeblock-last' }}).range(line.from));
                continue;
            }} else if (inCodeBlock) {{
                decs.push(Decoration.line({{ class: 'cm-codeblock-line' }}).range(line.from));
                continue;
            }}

            // Tables
            if (trimmed.startsWith('|') && trimmed.endsWith('|')) {{
                if (!inTable) {{ inTable = true; isHeader = true; }}
                if (trimmed.match(/^\|[\s\-:|]+\|$/)) {{
                    decs.push(Decoration.line({{ class: 'cm-table-separator' }}).range(line.from));
                    isHeader = false;
                }} else if (isHeader) {{
                    decs.push(Decoration.line({{ class: 'cm-table-header' }}).range(line.from));
                }} else {{
                    decs.push(Decoration.line({{ class: 'cm-table-row' }}).range(line.from));
                }}
                continue;
            }} else {{
                inTable = false; isHeader = true;
            }}

            // Task checkboxes are owned by the editor compatibility adapter's
            // taskListExtension. Keeping a second renderer here referenced a
            // function-local TaskCheckWidget that is not in this script's
            // scope and crashed editor initialization for every note.
            if (text.match(/^(\s*[-*]\s*)\[([ xX])\]\s/)) {{
                continue;
            }}

            // Wikilinks: [[target]] or [[target|display]] — add link styling
            let wlIdx = 0, wlSafety = 0;
            while ((wlIdx = text.indexOf('[[', wlIdx)) !== -1 && wlSafety++ < 20) {{
                // Skip embed syntax ![[
                if (wlIdx > 0 && text[wlIdx - 1] === '!') {{ wlIdx += 2; continue; }}
                const wlEnd = text.indexOf(']]', wlIdx + 2);
                if (wlEnd > wlIdx) {{
                    decs.push(Decoration.mark({{ class: 'cm-wikilink' }}).range(line.from + wlIdx, line.from + wlEnd + 2));
                    wlIdx = wlEnd + 2;
                }} else break;
            }}

            // Embeds are rendered by FlyntEditorCompat's embedExtension,
            // which owns resolution and navigation. Do not duplicate that
            // widget here; its EmbedWidget is intentionally function-local.
            if (trimmed.match(/^!\[\[(.+?)\]\]$/)) {{
                continue;
            }}
        }}
        return Decoration.set(decs, true);
    }});

    // ── Wikilink bracket hiding — lightweight, selection-aware ──
    // Separate from combinedPlugin so structural decorations don't recompute on cursor move.
    const wikilinkHidePlugin = EditorView.decorations.compute(['doc', 'selection'], (state) => {{
        const decs = [];
        const doc = state.doc;
        const sel = state.selection.main;
        const activeLine = doc.lineAt(sel.head).number;

        for (let i = 1; i <= doc.lines; i++) {{
            if (i === activeLine) continue; // show raw syntax on active line
            const line = doc.line(i);
            const text = line.text;
            let idx = 0, safety = 0;
            while ((idx = text.indexOf('[[', idx)) !== -1 && safety++ < 20) {{
                // Skip embed syntax ![[
                if (idx > 0 && text[idx - 1] === '!') {{ idx += 2; continue; }}
                const end = text.indexOf(']]', idx + 2);
                if (end > idx) {{
                    const inner = text.substring(idx + 2, end);
                    const pipe = inner.indexOf('|');
                    // Hide opening [[
                    decs.push(Decoration.replace({{}}).range(line.from + idx, line.from + idx + 2));
                    // For [[target|display]], also hide target and pipe
                    if (pipe >= 0) {{
                        decs.push(Decoration.replace({{}}).range(line.from + idx + 2, line.from + idx + 2 + pipe + 1));
                    }}
                    // Hide closing ]]
                    decs.push(Decoration.replace({{}}).range(line.from + end, line.from + end + 2));
                    idx = end + 2;
                }} else break;
            }}
        }}
        return Decoration.set(decs, true);
    }});

    // Legacy — kept for reference but NOT used (replaced by combinedPlugin)
    const tablePlugin_unused = EditorView.decorations.compute(['doc'], (state) => {{
        const decs = [];
        const doc = state.doc;
        let inTable = false;
        let isHeader = true;
        for (let i = 1; i <= doc.lines; i++) {{
            const line = doc.line(i);
            const t = line.text.trim();
            if (t.startsWith('|') && t.endsWith('|')) {{
                if (!inTable) {{ inTable = true; isHeader = true; }}
                // Separator line (|---|---|)
                if (t.match(/^\|[\s\-:|]+\|$/)) {{
                    decs.push(Decoration.line({{ class: 'cm-table-sep' }}).range(line.from));
                    isHeader = false;
                }} else if (isHeader) {{
                    decs.push(Decoration.line({{ class: 'cm-table-header' }}).range(line.from));
                }} else {{
                    decs.push(Decoration.line({{ class: 'cm-table-row' }}).range(line.from));
                }}
            }} else {{
                inTable = false;
                isHeader = true;
            }}
        }}
        return Decoration.set(decs);
    }});

    const codeBlockPlugin = EditorView.decorations.compute(['doc'], (state) => {{
        const decorations = [];
        const doc = state.doc;
        let inBlock = false;
        for (let i = 1; i <= doc.lines; i++) {{
            const line = doc.line(i);
            const text = line.text.trimStart();
            if (!inBlock && text.startsWith('```')) {{
                inBlock = true;
                decorations.push(Decoration.line({{ class: 'cm-codeblock-fence cm-codeblock-first' }}).range(line.from));
            }} else if (inBlock && text.startsWith('```')) {{
                inBlock = false;
                decorations.push(Decoration.line({{ class: 'cm-codeblock-fence cm-codeblock-last' }}).range(line.from));
            }} else if (inBlock) {{
                decorations.push(Decoration.line({{ class: 'cm-codeblock-line' }}).range(line.from));
            }}
        }}
        return Decoration.set(decorations);
    }});

    const flyntEmbedResolver = window.FlyntEmbedResolver;

    // Save on blur / visibility change — never lose content
    document.addEventListener('visibilitychange', () => {{
        if (document.hidden && window.FlyntEditor) {{
            window._flyntNotify('autosave', window.FlyntEditor.getDocument().content);
        }}
    }});
    window.addEventListener('blur', () => {{
        if (window.FlyntEditor) {{
            window._flyntNotify('autosave', window.FlyntEditor.getDocument().content);
        }}
    }});

    const flyntKeymaps = window.FlyntEditorCompat && window.FlyntEditorCompat.keymapRegistry
        ? window.FlyntEditorCompat.keymapRegistry(keymap)
        : (function() {{
            function wrapSelection(view, before, after) {{
                const sel = view.state.selection.main;
                const selected = view.state.sliceDoc(sel.from, sel.to);
                if (selected.startsWith(before) && selected.endsWith(after)) {{
                    view.dispatch({{ changes: {{ from: sel.from, to: sel.to, insert: selected.slice(before.length, -after.length) }} }});
                }} else {{
                    view.dispatch({{ changes: {{ from: sel.from, to: sel.to, insert: before + selected + after }} }});
                }}
                return true;
            }}
            return {{
                save: keymap.of([
                    {{ key: 'Mod-s', run: (view) => {{ window._flyntNotify('save', view.state.doc.toString()); return true; }} }},
                    {{ key: 'Mod-e', run: () => {{ window._flyntNotify('mode', 'source'); return true; }} }},
                ]),
                formatting: keymap.of([
                    {{ key: 'Mod-b', run: (v) => wrapSelection(v, '**', '**') }},
                    {{ key: 'Mod-i', run: (v) => wrapSelection(v, '*', '*') }},
                    {{ key: 'Mod-k', run: (v) => {{
                        const sel = v.state.selection.main;
                        const selected = v.state.sliceDoc(sel.from, sel.to);
                        v.dispatch({{ changes: {{ from: sel.from, to: sel.to, insert: '[' + selected + '](url)' }} }});
                        return true;
                    }} }},
                ]),
            }};
        }})();

    // Context menu action dispatcher — delegates to the centralized editor command registry.
    window._flyntCtxAction = function(id, view) {{
        if (window.FlyntEditor && typeof window.FlyntEditor.executeCommand === 'function') {{
            window.FlyntEditor.executeCommand(id);
            return;
        }}
        if (window.FlyntEditorCompat && typeof window.FlyntEditorCompat.dispatchEditorCommand === 'function') {{
            window.FlyntEditorCompat.dispatchEditorCommand(id);
            return;
        }}
        view.focus();
    }};

    const docText = {escaped};
    // Place cursor after frontmatter (first blank line after +++ closing)
    let cursorPos = docText.length;
    const fmMatch = docText.match(/^\+\+\+\n[\s\S]*?\n\+\+\+\n/);
    if (fmMatch) {{
        cursorPos = fmMatch[0].length;
        // Skip any blank lines after frontmatter
        while (cursorPos < docText.length && docText[cursorPos] === '\n') cursorPos++;
    }}
    const flyntLocalExtensions = [
                livePreview,
                hideMarkupPlugin,
                combinedPlugin,
                wikilinkHidePlugin,
                codeBlockPlugin,
                window.FlyntEditorCompat.embedExtension({{ EditorView, Decoration, WidgetType }}, flyntEmbedResolver),
                window.FlyntEditorCompat.contextMenuExtension(EditorView),
                window.FlyntEditorCompat.wikilinkInteractionExtension(EditorView),
            ];

    if (!window.FlyntEditorCompat || typeof window.FlyntEditorCompat.mountEditor !== 'function') {{
        throw new Error('Editor bridge bundle unavailable: FlyntEditorCompat.mountEditor missing');
    }}
    window.FlyntEditorCompat.mountEditor({{
        EditorState,
        EditorView,
        syntaxHighlighting,
        flyntHighlight,
        defaultHighlightStyle,
        oneDark,
        highlightActiveLine,
        highlightSpecialChars,
        highlightSelectionMatches,
        drawSelection,
        bracketMatching,
        closeBrackets,
        searchKeymap,
        defaultKeymap,
        history,
        historyKeymap,
        indentWithTab,
        markdown,
        markdownLanguage,
        GFM,
        languages,
        createFrontmatterHider,
        createBlockRender,
        Decoration,
        WidgetType,
        keymap,
    }}, container, docText, cursorPos, flyntLocalExtensions, flyntTheme);

    if (window.FlyntEditor) window.FlyntEditor.focus();
    console.timeEnd('cm6-init');
    console.timeEnd('cm6-total');
    }} // end _initCM
    try {{ _initCM(); }} catch(e) {{
        const c = document.getElementById('flynt-cm-editor');
        if (c) {{
            c.innerHTML = '<pre style="color:#ef4444;padding:20px;font-size:12px;white-space:pre-wrap;">CM6 error: ' + e.message + '\n\n' + (e.stack || '') + '</pre>';
        }}
        if (window._flyntNotify) window._flyntNotify('debug', 'CM6_ERROR: ' + e.message);
    }}
}})();
"#
    )
}

// ── Notification bridge JS ──────────────────────────────────────────────────
// Uses a global function + polling eval to decouple CM6 lifecycle from
// the Dioxus eval channel. CM6 calls window._flyntNotify(type, data),
// which queues messages. A persistent eval loop drains the queue.

const BRIDGE_JS: &str = r#"
if (!window._flyntQueue) {
    window._flyntQueue = [];
    window._flyntNotify = function(type, data) {
        window._flyntQueue.push(JSON.stringify({type: type, data: data, doc_id: window._flyntActiveDocId || null}));
    };
}

// Drain loop — sends queued messages to Rust via this eval's channel
async function _flyntDrain() {
    while (true) {
        if (window._flyntQueue.length > 0) {
            const msg = window._flyntQueue.shift();
            dioxus.send(msg);
        } else {
            await new Promise(r => setTimeout(r, 50));
        }
    }
}
_flyntDrain();

// Click-to-edit for Excalidraw embeds
document.addEventListener('click', function(e) {
    const embed = e.target.closest('.excalidraw-embed[data-drawing]');
    if (embed) {
        window._flyntNotify('preview-clear', '');
        const drawing = embed.getAttribute('data-drawing');
        if (drawing) {
            window._flyntNotify('open-drawing', drawing);
        }
    }
    const note = e.target.closest('a[data-flynt-note]');
    if (note) {
        e.preventDefault();
        window._flyntNotify('preview-clear', '');
        const slug = note.getAttribute('data-flynt-note');
        if (slug) {
            window._flyntNotify('nav', slug);
        }
    }
});

document.addEventListener('mouseover', function(e) {
    const note = e.target.closest('a[data-flynt-note]');
    if (!note || note._flyntPreviewArmed) return;
    note._flyntPreviewArmed = true;
    const slug = note.getAttribute('data-flynt-note');
    const timer = setTimeout(function() {
        if (!note.matches(':hover')) return;
        window._flyntNotify('preview-note', JSON.stringify({
            slug: slug,
            x: e.clientX,
            y: e.clientY
        }));
    }, 450);
    note._flyntPreviewTimer = timer;
});

document.addEventListener('mouseout', function(e) {
    const note = e.target.closest('a[data-flynt-note]');
    if (!note) return;
    if (e.relatedTarget && note.contains(e.relatedTarget)) return;
    if (note._flyntPreviewTimer) clearTimeout(note._flyntPreviewTimer);
    note._flyntPreviewArmed = false;
    window._flyntNotify('preview-clear', '');
});

document.addEventListener('keydown', function(e) {
    if (e.key === 'Escape') {
        window._flyntNotify('preview-clear', '');
    }
});
"#;

#[component]
fn NoteInspector(
    tab: Signal<InspectorTab>,
    body: String,
    frontmatter: Frontmatter,
    link_context: Option<LinkContext>,
    on_open_doc: EventHandler<DocumentMeta>,
    on_jump_line: EventHandler<usize>,
    on_publication_change: EventHandler<PublicationEdit>,
    on_publish_preview: EventHandler<()>,
    on_close: EventHandler<()>,
) -> Element {
    let headings = extract_headings(&body);
    rsx! {
        aside { class: "note-inspector",
            div { class: "note-inspector-header",
                div { class: "note-inspector-title", "Context" }
                button {
                    class: "note-inspector-close",
                    title: "Close inspector",
                    onclick: move |_| on_close.call(()),
                    "\u{00D7}"
                }
            }
            div { class: "note-inspector-tabs",
                InspectorTabButton { tab, value: InspectorTab::Links, label: "Links" }
                InspectorTabButton { tab, value: InspectorTab::Outline, label: "Outline" }
                InspectorTabButton { tab, value: InspectorTab::Properties, label: "Properties" }
            }
            div { class: "note-inspector-body",
                match *tab.read() {
                    InspectorTab::Links => rsx! {
                        NoteLinksPanel {
                            link_context,
                            on_open_doc,
                        }
                    },
                    InspectorTab::Outline => rsx! {
                        NoteOutlinePanel {
                            headings,
                            on_jump_line,
                        }
                    },
                    InspectorTab::Properties => rsx! {
                        NotePropertiesPanel {
                            frontmatter,
                            on_publication_change,
                            on_publish_preview,
                        }
                    },
                }
            }
        }
    }
}

#[component]
fn InspectorTabButton(
    tab: Signal<InspectorTab>,
    value: InspectorTab,
    label: &'static str,
) -> Element {
    let active = *tab.read() == value;
    rsx! {
        button {
            class: if active { "note-inspector-tab active" } else { "note-inspector-tab" },
            onclick: move |_| *tab.write() = value,
            "{label}"
        }
    }
}

#[component]
fn NoteLinksPanel(
    link_context: Option<LinkContext>,
    on_open_doc: EventHandler<DocumentMeta>,
) -> Element {
    match link_context {
        None => rsx! { div { class: "note-inspector-empty", "Loading links..." } },
        Some(ctx) => rsx! {
            div { class: "note-link-summary",
                div { class: "note-link-stat",
                    span { class: "note-link-stat-value", "{ctx.backlinks.len()}" }
                    span { class: "note-link-stat-label", "backlinks" }
                }
                div { class: "note-link-stat",
                    span { class: "note-link-stat-value", "{ctx.resolved_count}" }
                    span { class: "note-link-stat-label", "resolved" }
                }
                div { class: "note-link-stat missing",
                    span { class: "note-link-stat-value", "{ctx.missing_count}" }
                    span { class: "note-link-stat-label", "missing" }
                }
            }
            if !ctx.aliases.is_empty() {
                div { class: "note-inspector-section compact",
                    div { class: "note-inspector-section-title", "Accepted aliases" }
                    div { class: "note-property-chips",
                        for alias in ctx.aliases {
                            span { class: "note-property-chip", "{alias}" }
                        }
                    }
                }
            }
            div { class: "note-inspector-section",
                div { class: "note-inspector-section-title",
                    "Backlinks"
                    span { class: "note-inspector-count", "{ctx.backlinks.len()}" }
                }
                if ctx.backlinks.is_empty() {
                    div { class: "note-inspector-empty", "No backlinks" }
                } else {
                    div { class: "note-inspector-list",
                        for doc in ctx.backlinks {
                            {
                                let meta = doc.clone();
                                rsx! {
                                    button {
                                        class: "note-inspector-item",
                                        onclick: move |_| on_open_doc.call(meta.clone()),
                                        span { class: "note-inspector-item-title", "{doc.title}" }
                                        span { class: "note-inspector-item-meta", "{doc.path.display()}" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            div { class: "note-inspector-section",
                div { class: "note-inspector-section-title",
                    "Outgoing"
                    span { class: "note-inspector-count", "{ctx.outgoing.len()}" }
                }
                if ctx.outgoing.is_empty() {
                    div { class: "note-inspector-empty", "No outgoing links" }
                } else {
                    div { class: "note-inspector-list",
                        for link in ctx.outgoing {
                            {
                                let label = link.display.clone().unwrap_or_else(|| link.target.clone());
                                let anchor = link.anchor.clone();
                                let meta = link.resolved.clone();
                                let artifact = link.resolved_artifact.clone();
                                let disabled = meta.is_none();
                                let meta_for_open = meta.clone();
                                let status = if link.resolved.is_some() || artifact.is_some() { "resolved" } else { "missing" };
                                let mut classes = "note-inspector-item link-target".to_string();
                                if disabled {
                                    classes.push_str(" missing");
                                }
                                rsx! {
                                    button {
                                        class: "{classes}",
                                        disabled,
                                        onclick: move |_| {
                                            if let Some(doc) = meta_for_open.clone() {
                                                on_open_doc.call(doc);
                                            }
                                        },
                                        span { class: "note-inspector-item-title", "{label}" }
                                        span { class: "note-inspector-item-meta",
                                            "{link.target}"
                                            if let Some(anchor) = anchor {
                                                " #{anchor}"
                                            }
                                            if link.count > 1 {
                                                " x{link.count}"
                                            }
                                        }
                                        span { class: "note-link-status {status}", "{status}" }
                                        if let Some(kind) = artifact {
                                            span { class: "note-inspector-item-meta", "Resolved {kind} artifact" }
                                        } else if disabled {
                                            span { class: "note-inspector-item-meta", "No matching note yet" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        },
    }
}

#[component]
fn NoteOutlinePanel(headings: Vec<NoteHeading>, on_jump_line: EventHandler<usize>) -> Element {
    if headings.is_empty() {
        return rsx! { div { class: "note-inspector-empty", "No headings" } };
    }
    rsx! {
        div { class: "note-inspector-list",
            for heading in headings {
                {
                    let line = heading.line;
                    let indent = ((heading.level.saturating_sub(1)) * 12).min(60);
                    rsx! {
                        button {
                            class: "note-inspector-item outline-item",
                            style: "padding-left: calc(var(--space-2) + {indent}px);",
                            onclick: move |_| on_jump_line.call(line),
                            span { class: "note-inspector-item-title", "{heading.title}" }
                            span { class: "note-inspector-item-meta", "line {heading.line} · #{heading.anchor}" }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn NotePropertiesPanel(
    frontmatter: Frontmatter,
    on_publication_change: EventHandler<PublicationEdit>,
    on_publish_preview: EventHandler<()>,
) -> Element {
    let kind = frontmatter
        .kind
        .clone()
        .unwrap_or_else(|| "document".into());
    let status = frontmatter.status.clone().unwrap_or_else(|| "none".into());
    let id = frontmatter
        .id
        .map(|id| id.to_string())
        .unwrap_or_else(|| "unmanaged".into());
    let publication = frontmatter.publication.clone();
    let publication_status = if publication.enabled {
        format!("{:?}", publication.visibility).to_lowercase()
    } else {
        "disabled".into()
    };
    let slug = publication.slug.clone().unwrap_or_default();
    let collections = publication.collections.join(", ");
    let data_rows = frontmatter_data_rows(&frontmatter);

    rsx! {
        div { class: "note-properties",
            PropertyRow { label: "Kind", value: kind }
            PropertyRow { label: "Status", value: status }
            PropertyRow { label: "ID", value: id }
            PropertyRow { label: "Publication", value: publication_status }
            div { class: "note-property-block note-publication-editor",
                div { class: "note-property-label", "Publish" }
                label { class: "note-property-toggle",
                    input {
                        r#type: "checkbox",
                        checked: publication.enabled,
                        onchange: move |e| on_publication_change.call(PublicationEdit::Enabled(e.checked())),
                    }
                    span { "Enabled for export" }
                }
                div { class: "note-property-control-grid",
                    label {
                        span { class: "note-property-control-label", "Visibility" }
                        select {
                            class: "note-property-input",
                            value: "{visibility_value(publication.visibility)}",
                            onchange: move |e| {
                                on_publication_change.call(PublicationEdit::Visibility(parse_visibility(&e.value())));
                            },
                            option { value: "private", "private" }
                            option { value: "unlisted", "unlisted" }
                            option { value: "public", "public" }
                        }
                    }
                    label {
                        span { class: "note-property-control-label", "Slug" }
                        input {
                            class: "note-property-input",
                            value: "{slug}",
                            placeholder: "auto from title",
                            onchange: move |e| on_publication_change.call(PublicationEdit::Slug(e.value())),
                        }
                    }
                    label {
                        span { class: "note-property-control-label", "Collections" }
                        input {
                            class: "note-property-input",
                            value: "{collections}",
                            placeholder: "guide, release-notes",
                            onchange: move |e| on_publication_change.call(PublicationEdit::Collections(e.value())),
                        }
                    }
                }
                button {
                    class: "btn btn-ghost btn-sm",
                    onclick: move |_| on_publish_preview.call(()),
                    "Export preview"
                }
            }
            div { class: "note-property-block",
                div { class: "note-property-label", "Tags" }
                if frontmatter.tags.is_empty() {
                    div { class: "note-property-empty", "none" }
                } else {
                    div { class: "note-property-chips",
                        for tag in frontmatter.tags {
                            span { class: "note-property-chip", "#{tag}" }
                        }
                    }
                }
            }
            div { class: "note-property-block",
                div { class: "note-property-label", "Aliases" }
                if frontmatter.aliases.is_empty() {
                    div { class: "note-property-empty", "none" }
                } else {
                    div { class: "note-property-chips",
                        for alias in frontmatter.aliases {
                            span { class: "note-property-chip", "{alias}" }
                        }
                    }
                }
            }
            if !data_rows.is_empty() {
                div { class: "note-property-block",
                    div { class: "note-property-label", "[data]" }
                    div { class: "note-property-data",
                        for (key, value) in data_rows {
                            div { class: "note-property-data-row",
                                span { class: "note-property-data-key", "{key}" }
                                span { class: "note-property-data-value", "{value}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn PropertyRow(label: &'static str, value: String) -> Element {
    rsx! {
        div { class: "note-property-row",
            span { class: "note-property-label", "{label}" }
            span { class: "note-property-value", "{value}" }
        }
    }
}

fn frontmatter_data_rows(frontmatter: &Frontmatter) -> Vec<(String, String)> {
    let Some(data) = frontmatter.data.as_ref().and_then(|v| v.as_table()) else {
        return vec![];
    };
    data.iter()
        .map(|(key, value)| (key.clone(), compact_toml_value(value)))
        .collect()
}

fn visibility_value(visibility: PublicationVisibility) -> &'static str {
    match visibility {
        PublicationVisibility::Private => "private",
        PublicationVisibility::Unlisted => "unlisted",
        PublicationVisibility::Public => "public",
    }
}

fn parse_visibility(value: &str) -> PublicationVisibility {
    match value {
        "public" => PublicationVisibility::Public,
        "unlisted" => PublicationVisibility::Unlisted,
        _ => PublicationVisibility::Private,
    }
}

fn parse_collections(value: &str) -> Vec<String> {
    value
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}

fn apply_publication_edit(
    mut publication: PublicationConfig,
    edit: PublicationEdit,
) -> PublicationConfig {
    match edit {
        PublicationEdit::Enabled(enabled) => publication.enabled = enabled,
        PublicationEdit::Visibility(visibility) => publication.visibility = visibility,
        PublicationEdit::Slug(value) => {
            let value = value.trim().trim_matches('/').to_string();
            publication.slug = if value.is_empty() { None } else { Some(value) };
        }
        PublicationEdit::Collections(value) => publication.collections = parse_collections(&value),
    }
    publication
}

fn compact_toml_value(value: &toml::Value) -> String {
    match value {
        toml::Value::String(s) => s.clone(),
        toml::Value::Integer(v) => v.to_string(),
        toml::Value::Float(v) => v.to_string(),
        toml::Value::Boolean(v) => v.to_string(),
        toml::Value::Datetime(v) => v.to_string(),
        toml::Value::Array(values) => {
            let items = values.iter().map(compact_toml_value).collect::<Vec<_>>();
            format!("[{}]", items.join(", "))
        }
        toml::Value::Table(_) => "{...}".into(),
    }
}

#[component]
fn NoteHistoryModal(
    path: std::path::PathBuf,
    state: Option<HistoryPanelState>,
    snapshot: Option<FileSnapshot>,
    current_body: String,
    snapshot_error: Option<String>,
    restore_message: Option<String>,
    on_close: EventHandler<()>,
    on_select_commit: EventHandler<String>,
    on_restore_snapshot: EventHandler<FileSnapshot>,
) -> Element {
    let diff_lines = snapshot
        .as_ref()
        .map(|snapshot| build_line_diff(&snapshot.content, &current_body))
        .unwrap_or_default();

    rsx! {
        div { class: "history-overlay", onclick: move |_| on_close.call(()) }
        div { class: "history-modal", onclick: move |e| e.stop_propagation(),
            div { class: "history-header",
                div {
                    div { class: "history-title", "Note History" }
                    div { class: "history-path", "{path.display()}" }
                }
                button {
                    class: "note-inspector-close",
                    title: "Close history",
                    onclick: move |_| on_close.call(()),
                    "\u{00D7}"
                }
            }
            div { class: "history-body",
                div { class: "history-list",
                    match state {
                        None => rsx! { div { class: "note-inspector-empty", "Loading history..." } },
                        Some(HistoryPanelState { error: Some(error), .. }) => rsx! {
                            div { class: "history-error", "{error}" }
                        },
                        Some(HistoryPanelState { entries, error: None }) => rsx! {
                            if entries.is_empty() {
                                div { class: "note-inspector-empty", "No commits found for this note" }
                            } else {
                                for entry in entries {
                                    {
                                        let commit = entry.commit.clone();
                                        let timestamp = entry.timestamp.format("%Y-%m-%d %H:%M").to_string();
                                        rsx! {
                                            button {
                                                class: "history-entry",
                                                onclick: move |_| on_select_commit.call(commit.clone()),
                                                span { class: "history-entry-summary", "{entry.summary}" }
                                                span { class: "history-entry-meta",
                                                    "{entry.short_commit} · {entry.author} · {timestamp}"
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        },
                    }
                }
                div { class: "history-preview",
                    if let Some(message) = restore_message {
                        div { class: "history-restore-message", "{message}" }
                    }
                    if let Some(error) = snapshot_error {
                        div { class: "history-error", "{error}" }
                    }
                    if let Some(snapshot) = snapshot {
                        div { class: "history-preview-toolbar",
                            div {
                                div { class: "history-entry-meta", "{snapshot.commit.chars().take(7).collect::<String>()}" }
                                div { class: "history-diff-caption", "Selected commit compared to current note" }
                            }
                            {
                                let restore_snapshot = snapshot.clone();
                                rsx! {
                                    button {
                                        class: "btn btn-primary btn-sm",
                                        onclick: move |_| on_restore_snapshot.call(restore_snapshot.clone()),
                                        "Restore as copy"
                                    }
                                }
                            }
                        }
                        div { class: "history-diff-content",
                            if diff_lines.is_empty() {
                                div { class: "history-preview-empty", "Snapshot and current note are identical." }
                            } else {
                                for line in diff_lines {
                                    {
                                        let class = match line.kind {
                                            HistoryDiffKind::Context => "context",
                                            HistoryDiffKind::Added => "added",
                                            HistoryDiffKind::Removed => "removed",
                                        };
                                        let marker = match line.kind {
                                            HistoryDiffKind::Context => " ",
                                            HistoryDiffKind::Added => "+",
                                            HistoryDiffKind::Removed => "-",
                                        };
                                        let old_line = line.old_line.map(|n| n.to_string()).unwrap_or_default();
                                        let new_line = line.new_line.map(|n| n.to_string()).unwrap_or_default();
                                        rsx! {
                                            div { class: "history-diff-line {class}",
                                                span { class: "history-diff-gutter old", "{old_line}" }
                                                span { class: "history-diff-gutter new", "{new_line}" }
                                                span { class: "history-diff-marker", "{marker}" }
                                                code { class: "history-diff-text", "{line.text}" }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    } else {
                        div { class: "history-preview-empty", "Select a commit to preview the note body at that point." }
                    }
                }
            }
        }
    }
}

#[component]
fn PublishPreviewModal(
    state: Option<PublishPreviewState>,
    error: Option<String>,
    on_close: EventHandler<()>,
) -> Element {
    rsx! {
        div { class: "history-overlay", onclick: move |_| on_close.call(()) }
        div { class: "history-modal publish-preview-modal", onclick: move |e| e.stop_propagation(),
            div { class: "history-header",
                div {
                    div { class: "history-title", "Publication Preview" }
                    div { class: "history-path", "Local static export report" }
                }
                button {
                    class: "note-inspector-close",
                    title: "Close publication preview",
                    onclick: move |_| on_close.call(()),
                    "\u{00D7}"
                }
            }
            div { class: "publish-report-body",
                if let Some(error) = error {
                    div { class: "history-error", "{error}" }
                } else if let Some(state) = state {
                    div { class: "publish-report-grid",
                        div { class: "publish-report-stat",
                            span { class: "publish-report-label", "Exported" }
                            strong { "{state.exported}" }
                        }
                        div { class: "publish-report-stat",
                            span { class: "publish-report-label", "Skipped private" }
                            strong { "{state.skipped_private}" }
                        }
                        div { class: "publish-report-stat",
                            span { class: "publish-report-label", "Errors" }
                            strong { "{state.errors.len()}" }
                        }
                    }
                    div { class: "note-property-block",
                        div { class: "note-property-label", "Output" }
                        code { class: "publish-report-path", "{state.output_path.display()}" }
                    }
                    if !state.errors.is_empty() {
                        div { class: "note-property-block",
                            div { class: "note-property-label", "Errors" }
                            div { class: "history-diff-content",
                                for error in state.errors {
                                    div { class: "history-error", "{error}" }
                                }
                            }
                        }
                    }
                } else {
                    div { class: "note-inspector-empty", "Exporting publication preview..." }
                }
            }
        }
    }
}

// ── Notes view ──────────────────────────────────────────────────────────────

#[component]
pub fn NotesView() -> Element {
    let ctx = use_context::<AppContext>();
    let mut tab_state = use_context::<Signal<TabState>>();
    let mut is_drawing = use_context::<Signal<bool>>();
    let ctx_res = ctx.clone();
    let ctx_save2 = ctx.clone();

    let mut mode = use_signal(|| EditMode::Live);
    let mut diagram_zoom = use_signal(|| 1.0_f32);
    let mut edit_body = use_signal(String::new);
    let mut save_err = use_signal(|| Option::<String>::None);
    let mut save_state = use_signal(|| SaveState::Clean);
    let mut render_ver = use_signal(|| 0u32);
    let mut conflict_detected = use_signal(|| false);
    let mut inspector_open = use_signal(|| true);
    let mut inspector_tab = use_signal(|| InspectorTab::Links);
    let mut history_open = use_signal(|| false);
    let mut history_snapshot: Signal<Option<FileSnapshot>> = use_signal(|| None);
    let mut history_snapshot_error: Signal<Option<String>> = use_signal(|| None);
    let mut history_restore_message: Signal<Option<String>> = use_signal(|| None);
    let mut publish_preview_open = use_signal(|| false);
    let mut publish_preview_state: Signal<Option<PublishPreviewState>> = use_signal(|| None);
    let mut publish_preview_error: Signal<Option<String>> = use_signal(|| None);
    let mut hover_preview: Signal<Option<HoverPreviewState>> = use_signal(|| None);
    let inspector_command = use_context::<Signal<NoteInspectorCommand>>();
    let mut last_inspector_command = use_signal(|| 0u64);
    let history_command = use_context::<Signal<NoteHistoryCommand>>();
    let mut last_history_command = use_signal(|| 0u64);
    let publication_preview_command = use_context::<Signal<PublicationPreviewCommand>>();
    let mut last_publication_preview_command = use_signal(|| 0u64);

    use_effect(move || {
        let command = *inspector_command.read();
        if command.version == *last_inspector_command.peek() {
            return;
        }
        *last_inspector_command.write() = command.version;
        match command.target {
            NoteInspectorTarget::Toggle => {
                let open = *inspector_open.peek();
                *inspector_open.write() = !open;
            }
            NoteInspectorTarget::Links => {
                *inspector_open.write() = true;
                *inspector_tab.write() = InspectorTab::Links;
            }
            NoteInspectorTarget::Outline => {
                *inspector_open.write() = true;
                *inspector_tab.write() = InspectorTab::Outline;
            }
            NoteInspectorTarget::Properties => {
                *inspector_open.write() = true;
                *inspector_tab.write() = InspectorTab::Properties;
            }
        }
    });

    use_effect(move || {
        let command = *history_command.read();
        if command.version == *last_history_command.peek() {
            return;
        }
        *last_history_command.write() = command.version;
        *history_snapshot.write() = None;
        *history_snapshot_error.write() = None;
        *history_restore_message.write() = None;
        *history_open.write() = true;
    });

    use_effect(move || {
        let command = *publication_preview_command.read();
        if command.version == *last_publication_preview_command.peek() {
            return;
        }
        *last_publication_preview_command.write() = command.version;
        *publish_preview_open.write() = true;
        *publish_preview_state.write() = None;
        *publish_preview_error.write() = None;
        let c = ctx.clone();
        spawn(async move {
            let project = c.project();
            let result = tokio::task::spawn_blocking(move || {
                crate::bootstrap::OmegonRuntimeContext::export_publication_preview_report(&project)
            })
            .await;
            match result {
                Ok(Ok((output_path, report))) => {
                    *publish_preview_state.write() = Some(PublishPreviewState {
                        output_path,
                        exported: report.exported,
                        skipped_private: report.skipped_private,
                        errors: report.errors,
                    });
                }
                Ok(Err(e)) => {
                    *publish_preview_error.write() = Some(format!("Publication export failed: {e}"))
                }
                Err(e) => {
                    *publish_preview_error.write() =
                        Some(format!("Publication export interrupted: {e}"))
                }
            }
        });
    });

    // ── Two-phase rendering ───────────────────────────────────────────
    // Phase 1 (instant): read document from SQLite synchronously — <1ms.
    //   Sets edit_body and raw content immediately so the editor is responsive.
    // Phase 2 (background): render HTML via comrak + query execution.
    //   Swaps in when ready. Cached for instant tab switching.

    // Render cache: doc_id → (path, title, body, html, has_conflicts)
    let mut render_cache: Signal<
        std::collections::HashMap<
            flynt_core::models::DocumentId,
            (std::path::PathBuf, String, String, String, bool),
        >,
    > = use_signal(std::collections::HashMap::new);

    // Invalidate cache on save
    use_effect(move || {
        let _ver = *render_ver.read();
        if _ver > 0 {
            if let Some(id) = tab_state.read().active_id().cloned() {
                render_cache.write().remove(&id);
            }
        }
    });

    // Phase 1: synchronous document read — no spawn_blocking, no async overhead.
    //
    // Tuple holds (id, path, title, body, frontmatter). Carrying the id
    // alongside the rest is what the sync effect below uses to detect
    // "is this still the doc we just asked for?" — without it, a stale
    // doc_data value (from a previous tab) could be propagated to the
    // editor when the sync effect fires before doc_data has refreshed
    // for the newly active tab.
    let mut doc_data: Signal<
        Option<(
            flynt_core::models::DocumentId,
            std::path::PathBuf,
            String,
            String,
            flynt_core::models::Frontmatter,
            chrono::DateTime<chrono::Utc>,
            chrono::DateTime<chrono::Utc>,
        )>,
    > = use_signal(|| None);
    use_effect(move || {
        let _ver = *render_ver.read();
        let selected_id = tab_state.read().active_id().cloned();
        let previous_doc_id = doc_data
            .peek()
            .as_ref()
            .map(|(id, _, _, _, _, _, _)| id.clone());
        if previous_doc_id.as_ref() != selected_id.as_ref() {
            *hover_preview.write() = None;
        }
        let Some(doc_id) = selected_id else {
            *doc_data.write() = None;
            return;
        };
        if previous_doc_id.as_ref() != Some(&doc_id) {
            *doc_data.write() = None;
        }
        // Synchronous SQLite read — <1ms for any document
        let project = ctx_res.project();
        match project.store.get_document(&doc_id) {
            Ok(Some(doc)) => {
                *doc_data.write() = Some((
                    doc.id.clone(),
                    doc.path.clone(),
                    doc.title.clone(),
                    doc.content.clone(),
                    doc.frontmatter.clone(),
                    doc.created_at,
                    doc.updated_at,
                ));
            }
            Ok(None) => {
                tracing::warn!("doc_data_effect: doc {:?} not found in store", doc_id);
            }
            Err(e) => {
                tracing::warn!("doc_data_effect: store error for {:?}: {e}", doc_id);
            }
        }
    });

    // Boards + engagements caches for the metadata strip's pickers.
    // Refreshed by the existing `refresh` signal (sidebar bumps it on
    // any project event), so a kanban board rename or a new engagement
    // shows up in the picker without a tab toggle.
    let mut boards_cache: Signal<Vec<flynt_core::models::Board>> = use_signal(Vec::new);
    let mut engagements_cache: Signal<Vec<flynt_core::models::Engagement>> = use_signal(Vec::new);
    use_effect(move || {
        let _ver = *render_ver.read();
        let project = ctx_res.project();
        if let Ok(b) = project.store.list_boards() {
            *boards_cache.write() = b;
        }
        if let Ok(e) = project.store.list_engagements() {
            *engagements_cache.write() = e;
        }
    });

    // Install the dispatcher once at mount. Picker `on_change` events
    // flow into this channel; the spawned receiver translates them into
    // `Project::set_data_field` calls. Going through a channel rather
    // than direct ctx access keeps the strip's component scope free of
    // AppContext (Dioxus contexts are scope-bound; the picker is in a
    // different scope path than the apply site).
    use_effect(move || {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<
            crate::components::task_metadata_strip::FieldChangeRequest,
        >();
        crate::components::task_metadata_strip::install_dispatcher(tx);

        let project = ctx_res.project();
        let mut bump = render_ver;
        spawn(async move {
            while let Some(req) = rx.recv().await {
                let key_for_log = req.key.clone();
                let key = req.key;
                let value = req.value;
                let path = req.path;
                let project_for_blocking = project.clone();
                let result = tokio::task::spawn_blocking(move || {
                    let toml_value =
                        crate::components::task_metadata_strip::translate_value(&key, &value);
                    project_for_blocking.set_data_field(&path, &key, toml_value)
                })
                .await;
                match result {
                    Ok(Ok(())) => {
                        // Bump render_ver so doc_data + rendered re-fetch
                        // and the strip re-renders with the new value.
                        *bump.write() += 1;
                    }
                    Ok(Err(e)) => {
                        tracing::error!(error = %e, "set_data_field failed for {key_for_log}");
                    }
                    Err(e) => {
                        tracing::error!(error = %e, "set_data_field task panicked for {key_for_log}");
                    }
                }
            }
        });
    });

    // Phase 2: background HTML rendering — fires after doc_data is set
    let rendered: Resource<Option<(std::path::PathBuf, String, String, String, bool)>> =
        use_resource(move || {
            let _ver = *render_ver.read();
            let selected_id = tab_state.read().active_id().cloned();
            let project = ctx_res.project();
            async move {
                let Some(doc_id) = selected_id else {
                    return None;
                };

                // Cache hit — instant
                if let Some(cached) = render_cache.read().get(&doc_id) {
                    return Some(cached.clone());
                }

                // Background render — won't block the UI
                let cache_id = doc_id.clone();
                let result = tokio::task::spawn_blocking(move || {
                    project
                        .store
                        .get_document(&doc_id)
                        .ok()
                        .flatten()
                        .map(|doc| {
                            let html = render_html_with_store(
                                &doc.content,
                                Some(&*project.store),
                                Some(&project.root),
                            );
                            let has_conflicts =
                                flynt_core::conflict::has_conflict_markers(&doc.content);
                            (
                                doc.path.clone(),
                                doc.title.clone(),
                                doc.content.clone(),
                                html,
                                has_conflicts,
                            )
                        })
                })
                .await
                .ok()
                .flatten();

                if let Some(ref r) = result {
                    *conflict_detected.write() = r.4;
                    render_cache.write().insert(cache_id, r.clone());
                }
                result
            }
        });

    let link_ctx = ctx.clone();
    let link_context: Resource<Option<LinkContext>> = use_resource(move || {
        let _ver = *render_ver.read();
        let selected_id = tab_state.read().active_id().cloned();
        let project = link_ctx.project();
        async move {
            let Some(doc_id) = selected_id else {
                return None;
            };
            tokio::task::spawn_blocking(move || {
                let Some(doc) = project.store.get_document(&doc_id)? else {
                    return Ok(None);
                };
                let backlinks = project.store.get_backlinks(&doc_id).unwrap_or_default();
                let artifact_index = build_artifact_link_index(&project.root);
                let context = build_link_context(
                    backlinks,
                    &doc.content,
                    &doc.frontmatter,
                    |target| {
                        project
                            .store
                            .find_document_by_slug(&target.to_lowercase())
                            .ok()
                            .flatten()
                    },
                    |target| resolve_artifact_link(&artifact_index, target),
                );
                Ok::<_, anyhow::Error>(Some(context))
            })
            .await
            .ok()
            .and_then(Result::ok)
            .flatten()
        }
    });

    let history_ctx = ctx.clone();
    let history_state: Resource<Option<HistoryPanelState>> = use_resource(move || {
        let is_open = *history_open.read();
        let selected = doc_data
            .read()
            .as_ref()
            .map(|(_, path, _, _, _, _, _)| path.clone());
        let project = history_ctx.project();
        async move {
            if !is_open {
                return None;
            }
            let Some(path) = selected else {
                return Some(HistoryPanelState {
                    entries: vec![],
                    error: Some("No active note selected".into()),
                });
            };
            tokio::task::spawn_blocking(move || {
                let (remote, branch) = match &project.config.sync {
                    flynt_core::models::SyncConfig::Git { remote, branch, .. } => {
                        (remote.clone(), branch.clone())
                    }
                    _ => ("origin".into(), "main".into()),
                };
                let git = GitSync::new(project.root.clone(), remote, branch);
                match git.list_file_history(&path, 40) {
                    Ok(entries) => HistoryPanelState {
                        entries,
                        error: None,
                    },
                    Err(e) => HistoryPanelState {
                        entries: vec![],
                        error: Some(format!("Could not read git history: {e}")),
                    },
                }
            })
            .await
            .ok()
        }
    });

    // The signal that drives CM6 loading. Its (id, body) value is the
    // "last content we asked CM6 to display." The CM6 init effect
    // subscribes to it; whenever it changes, CM6 swaps to the new body.
    //
    // Single source of truth for de-dup — no separate `synced_doc_id`.
    // The previous shape gated on synced_doc_id alone, which prevented
    // post-save propagation: after a save, doc_data refreshed with new
    // content but `already_synced` was true so cm6_load_source was
    // never updated. CM6 kept the pre-save body; the operator's saved
    // edits never appeared in Live mode.
    let mut cm6_load_source: Signal<Option<(flynt_core::models::DocumentId, String)>> =
        use_signal(|| None);
    let _render_message = use_signal(|| Option::<String>::None);

    {
        let active_path = rendered
            .read()
            .as_ref()
            .and_then(|r| r.as_ref().map(|t| t.0.clone()))
            .or_else(|| {
                doc_data
                    .read()
                    .as_ref()
                    .map(|(_, path, _, _, _, _, _)| path.clone())
            });
        let active_is_d2 = active_path
            .as_ref()
            .is_some_and(|path| is_d2_path(path) || d2_embed_path(&edit_body.read()).is_some());
        if active_is_d2 && *mode.read() == EditMode::Live {
            mode.set(EditMode::Diagram);
        } else if !active_is_d2 && *mode.read() == EditMode::Diagram {
            // Diagram mode belongs to the document that selected it. When a
            // drawing/D2 tab closes and a markdown note becomes active again,
            // restore that note's normal live renderer instead of leaving the
            // shared NotesView mode stranded on an empty Diagram branch.
            mode.set(EditMode::Live);
        }
    }
    use_effect(move || {
        let current_id = tab_state.read().active_id().cloned();
        let Some(active_id) = current_id else { return };
        // Confirm doc_data is for this tab (not a stale value from
        // the previous tab's load). If the ids mismatch, bail and
        // wait for doc_data to refresh — the effect subscribes to
        // doc_data so it will re-fire.
        let body = match &*doc_data.read() {
            Some((doc_id, _, _, body, _, _, _)) if doc_id == &active_id => body.clone(),
            _ => return,
        };
        // De-dup. Three cases trigger a load:
        //   1. First load (no prior cm6_load_source) — propagate.
        //   2. Tab switch (id changed) — propagate.
        //   3. Same tab, body on disk changed AND operator hasn't
        //      typed anything we'd be clobbering. We detect the
        //      no-clobber condition by checking edit_body against
        //      the previously-loaded body — if they match, the user
        //      has no unsaved divergence and it's safe to refresh.
        let action = match &*cm6_load_source.peek() {
            None => Some("first-load"),
            Some((prev_id, _)) if prev_id != &active_id => Some("tab-switch"),
            Some((_, prev_body)) => {
                if body != *prev_body {
                    // Body on disk changed since last load. Safe to
                    // overwrite only if edit_body matches what we last
                    // loaded (no unsaved divergence in CM6 / textarea).
                    let eb = edit_body.peek().clone();
                    if eb == *prev_body {
                        Some("disk-changed-no-divergence")
                    } else if eb == body {
                        // edit_body matches new body — operator just
                        // saved their own edits. Propagate so CM6
                        // catches up to the saved content.
                        Some("save-propagation")
                    } else {
                        // edit_body has uncommitted divergence AND
                        // disk changed externally. Keep their work;
                        // they'll have to resolve manually.
                        None
                    }
                } else {
                    None
                }
            }
        };
        let Some(reason) = action else { return };
        tracing::debug!(
            "sync_effect: propagating ({}) active_id={:?} body_len={}",
            reason,
            active_id,
            body.len()
        );
        *edit_body.write() = body.clone();
        *save_state.write() = SaveState::Clean;
        *cm6_load_source.write() = Some((active_id, body));
    });

    let has_active = tab_state.read().active_id().is_some();

    // Initialize CM6 when: new document loaded OR mode switched back to Live.
    //
    // Subscribes to `cm6_load_source` so the effect runs with the exact
    // body that came from the doc load — no race against edit_body.
    // Also subscribes to `mode` so toggling Source → Live re-fires the
    // init (calling cm6_init_js's swap path with the latest content).
    let is_drawing_mode = use_context::<Signal<bool>>();
    let init_ctx = ctx.clone();
    use_effect(move || {
        let source = cm6_load_source.read().clone();
        let Some((doc_id, body)) = source else { return };
        if *is_drawing_mode.read() {
            return;
        }
        if !matches!(&*mode.read(), EditMode::Live) {
            return;
        }
        tracing::info!(
            "CM6 init effect triggered for doc_id={:?} body_len={}",
            doc_id,
            body.len()
        );
        // Keep tab activation hot: building the full embed index touches every
        // document and artifact, which makes large repositories wait seconds
        // before CodeMirror receives the selected note. The editor can mount
        // immediately with an empty index; unresolved embeds remain editable
        // text until a later indexed enhancement path is added.
        let _ = &init_ctx;
        document::eval(&cm6_init_js(&doc_id, &body, "{}"));
    });

    // Autosave for Source mode (textarea path). CM6 already has its own
    // autosave wired through the bridge; this gives Source-mode editing
    // the same behavior so operators don't have to ⌘S manually after
    // typing in the textarea.
    //
    // Debounced: each edit_body change resets a 1.5s timer; the save
    // fires only after the operator has been quiet that long. Skips
    // when edit_body matches what's already on disk (no actual diff).
    let mut autosave_token = use_signal(|| 0u64);
    let autosave_ctx = ctx.clone();
    use_effect(move || {
        let body = edit_body.read().clone();
        if !matches!(&*mode.read(), EditMode::Source) {
            return;
        }
        // Resolve the path from doc_data — we need the relative path
        // to save to. If doc_data isn't loaded yet, skip.
        let (disk_body, path, frontmatter) = match &*doc_data.peek() {
            Some((_, p, _, b, fm, _, _)) => (b.clone(), p.clone(), fm.clone()),
            None => return,
        };
        if crate::visual_artifact_surface::resolve_wrapper_surface(
            &autosave_ctx.project_root(),
            &path,
            &disk_body,
            &frontmatter,
        )
        .is_some()
        {
            return;
        }
        if body == disk_body {
            return;
        } // no diff vs. disk
        let token = autosave_token.peek().wrapping_add(1);
        *autosave_token.write() = token;
        let mut bump = render_ver;
        let mut state = save_state;
        let mut err = save_err;
        let c = autosave_ctx.clone();
        spawn(async move {
            tokio::time::sleep(Duration::from_millis(1500)).await;
            // Newer edit superseded this one — bail.
            if *autosave_token.peek() != token {
                return;
            }
            let project = c.project();
            let path_for_save = path.clone();
            let body_for_save = body.clone();
            match tokio::task::spawn_blocking(move || {
                project.save_document_content(&path_for_save, &body_for_save)
            })
            .await
            {
                Ok(Ok(())) => {
                    *bump.write() += 1;
                    *err.write() = None;
                    *state.write() = SaveState::Saved;
                }
                Ok(Err(e)) => *err.write() = Some(format!("Autosave failed — {e}")),
                Err(e) => *err.write() = Some(format!("Autosave interrupted — {e}")),
            }
        });
    });

    // Persistent message bridge — one eval that polls a global queue.
    // CM6 pushes messages to the queue; this loop drains them to Rust.
    let ctx_link = ctx.clone();
    let mut ts_link = tab_state;
    let mut ar_link = use_context::<Signal<Route>>();
    use_effect(move || {
        let mut eval = document::eval(BRIDGE_JS);
        let c = ctx_link.clone();

        spawn(async move {
            loop {
                let Ok(val) = eval.recv::<String>().await else {
                    break;
                };

                let Ok(msg) = serde_json::from_str::<serde_json::Value>(&val) else {
                    continue;
                };
                let msg_type = msg["type"].as_str().unwrap_or("");
                let data = msg["data"].as_str().unwrap_or("");
                let msg_doc_id = msg["doc_id"]
                    .as_str()
                    .and_then(|s| uuid::Uuid::parse_str(s).ok())
                    .map(DocumentId);
                let active_doc_id = ts_link.read().active_id().cloned();
                let message_matches_active = msg_doc_id.as_ref() == active_doc_id.as_ref();

                match msg_type {
                    "edit" => {
                        if !message_matches_active {
                            tracing::warn!(
                                ?msg_doc_id,
                                ?active_doc_id,
                                "dropping stale editor edit message"
                            );
                            continue;
                        }
                        // Keep edit_body in sync with CM6's live content.
                        // The CM6 div has a stable id (`flynt-cm-editor`), so
                        // Dioxus reconciles it as the same element on re-render
                        // — the editor instance is not torn down. The earlier
                        // shape avoided this write and relied on a CM6 read at
                        // toggle time, which raced against post-save CM6
                        // re-init: a stale CM6 (still showing N-1 before its
                        // re-init ran) was adopted into edit_body, reverting
                        // the operator's saved edits when they toggled back
                        // to Source. With this write, edit_body is the single
                        // source of truth for "the current document contents."
                        *edit_body.write() = data.to_string();
                        *save_state.write() = SaveState::Dirty;
                    }
                    "save" | "autosave" => {
                        if !message_matches_active {
                            tracing::warn!(
                                ?msg_doc_id,
                                ?active_doc_id,
                                "dropping stale editor save message"
                            );
                            continue;
                        }
                        let content = data.to_string();
                        let Some((doc_id, path, _title, disk_body, frontmatter, _, _)) =
                            doc_data.peek().clone()
                        else {
                            tracing::warn!(
                                ?msg_doc_id,
                                "dropping editor save with no loaded doc_data"
                            );
                            continue;
                        };
                        if Some(&doc_id) != msg_doc_id.as_ref() {
                            tracing::warn!(
                                ?msg_doc_id,
                                ?doc_id,
                                "dropping editor save for non-loaded doc"
                            );
                            continue;
                        }
                        if crate::visual_artifact_surface::resolve_wrapper_surface(
                            &c.project_root(),
                            &path,
                            &disk_body,
                            &frontmatter,
                        )
                        .is_some()
                        {
                            *save_err.write() = Some("Visual artifact wrappers are protected; use the artifact editor/source command instead.".into());
                            continue;
                        }
                        let project = c.project();
                        let content_for_save = content.clone();
                        match tokio::task::spawn_blocking(move || {
                            project.save_document_content(&path, &content_for_save)
                        })
                        .await
                        {
                            Ok(Ok(())) => {
                                let saved_content = serde_json::to_string(&content)
                                    .unwrap_or_else(|_| "\"\"".into());
                                document::eval(&format!(
                                    "document.querySelectorAll('.save-status').forEach(e => {{ e.textContent = 'saved'; e.className = 'save-status saved'; }}); if (window.FlyntEditor) window.FlyntEditor.markSaved(undefined, {});",
                                    saved_content
                                ));
                            }
                            Ok(Err(e)) => *save_err.write() = Some(format!("Could not save — {e}")),
                            Err(e) => *save_err.write() = Some(format!("Save interrupted — {e}")),
                        }
                    }
                    "mode" => {
                        if data == "source" {
                            // edit_body is already the source of truth —
                            // the "edit" message handler above keeps it in
                            // sync with CM6. No read-CM6 dance needed.
                            *mode.write() = EditMode::Source;
                        }
                    }
                    "editor.embed.open" => {
                        let Ok(resolution) = serde_json::from_str::<serde_json::Value>(data) else {
                            continue;
                        };
                        let surface = resolution["surface"].as_str().unwrap_or("");
                        let ref_value = resolution["canonicalPath"]
                            .as_str()
                            .or_else(|| resolution["title"].as_str())
                            .or_else(|| resolution["label"].as_str())
                            .or_else(|| resolution["ref"].as_str())
                            .unwrap_or("");
                        match surface {
                            "drawing" | "flow" | "canvas" | "note" => {
                                let slug = std::path::Path::new(ref_value)
                                    .file_stem()
                                    .and_then(|s| s.to_str())
                                    .unwrap_or(ref_value)
                                    .to_lowercase();
                                let project = c.project();
                                if let Ok(Some(meta)) = tokio::task::spawn_blocking(move || {
                                    project.store.find_document_by_slug(&slug)
                                })
                                .await
                                .unwrap_or(Ok(None))
                                {
                                    ts_link.write().open(meta.id.clone(), meta.title.clone());
                                    *ar_link.write() = Route::Notes;
                                }
                            }
                            _ => {}
                        }
                    }
                    "open-drawing" => {
                        // Open the excalidraw wrapper .md in a tab — NotesView
                        // detects the embed and renders ExcalidrawView automatically
                        let drawing_file = data.to_string();
                        let slug = drawing_file.replace(".excalidraw", "").to_lowercase();
                        let project = c.project();
                        if let Ok(Some(meta)) = tokio::task::spawn_blocking(move || {
                            project.store.find_document_by_slug(&slug)
                        })
                        .await
                        .unwrap_or(Ok(None))
                        {
                            ts_link.write().open(meta.id.clone(), meta.title.clone());
                            *ar_link.write() = Route::Notes;
                        }
                    }
                    "nav" => {
                        let slug = data.to_lowercase();
                        let project = c.project();
                        if let Ok(Some(meta)) = tokio::task::spawn_blocking(move || {
                            project.store.find_document_by_slug(&slug)
                        })
                        .await
                        .unwrap_or(Ok(None))
                        {
                            ts_link.write().open(meta.id.clone(), meta.title.clone());
                            *ar_link.write() = Route::Notes;
                        }
                    }
                    "preview-note" => {
                        let Ok(payload) = serde_json::from_str::<serde_json::Value>(data) else {
                            continue;
                        };
                        let slug = payload["slug"].as_str().unwrap_or("").to_string();
                        if slug.trim().is_empty() {
                            continue;
                        }
                        let x = payload["x"].as_f64().unwrap_or(0.0);
                        let y = payload["y"].as_f64().unwrap_or(0.0);
                        let project = c.project();
                        match tokio::task::spawn_blocking(move || {
                            NotePreview::load_by_slug(&project, &slug)
                        })
                        .await
                        {
                            Ok(Some(preview)) => {
                                *hover_preview.write() = Some(HoverPreviewState { preview, x, y });
                            }
                            _ => *hover_preview.write() = None,
                        }
                    }
                    "preview-clear" => {
                        *hover_preview.write() = None;
                    }
                    _ => {}
                }
            }
        });
    });

    // No tab open
    if !has_active {
        return rsx! {
            crate::components::TabBar {}
            div { class: "notes-empty",
                div { class: "notes-empty-content",
                    div { class: "notes-empty-icon", dangerous_inner_html: crate::icons::ICON_SCROLL }
                    p { "Select a note from the sidebar" }
                    p { class: "notes-empty-hint", "or press + to create a new one" }
                }
            }
        };
    }

    // Gate on doc_data (synchronous, instant) not rendered (async, slow).
    // The editor gets raw content immediately; HTML preview swaps in when ready.
    let Some((_doc_id, rel_path, title, body, frontmatter, created_at, updated_at)) =
        doc_data.read().clone()
    else {
        return rsx! {
            crate::components::TabBar {}
            if has_active {
                div { class: "notes-loading muted", "Loading…" }
            }
        };
    };

    // Raw Excalidraw source tabs are editor surfaces, not markdown notes.
    // Route them through the same full-bleed container used by wrappers so
    // the editor is not nested inside the note titlebar/scroll layout.
    if crate::views::excalidraw::is_excalidraw(&rel_path) {
        is_drawing.set(true);
        return rsx! {
            crate::components::TabBar {}
            div {
                style: "display:flex;flex-direction:column;flex:1;overflow:hidden;padding:0;min-height:0;height:100%;",
                crate::views::ExcalidrawView { key: "{rel_path.display()}", path: rel_path }
            }
        };
    }

    // Visual artifact wrappers render through a central surface resolver.
    // This preserves the current UX while moving Excalidraw/Design Board
    // dispatch out of ad hoc NotesView body parsing.
    if let Some(surface) = crate::visual_artifact_surface::resolve_wrapper_surface(
        &ctx.project_root(),
        &rel_path,
        &body,
        &frontmatter,
    ) {
        is_drawing.set(true);
        return match surface {
            crate::visual_artifact_surface::VisualArtifactSurface::ExcalidrawPreview {
                source_path,
            } => {
                let html = crate::excalidraw_preview::render_excalidraw_preview_html(
                    &ctx.project_root(),
                    &source_path,
                );
                rsx! {
                    crate::components::TabBar {}
                    div { class: "notes-workspace",
                        div { class: "notes-pane",
                            div { class: "note-titlebar",
                                h2 { class: "note-title", "{title}" }
                                div { class: "note-actions",
                                    button {
                                        class: "btn btn-ghost",
                                        onclick: move |_| {
                                            let target = flynt_core::visual_artifacts::VisualArtifactRef {
                                                kind: flynt_core::visual_artifacts::VisualArtifactKind::ExcalidrawDrawing,
                                                source_path: source_path.clone(),
                                            };
                                            let request = flynt_core::visual_artifacts::ArtifactActionRequest::edit(target);
                                            if let Some((id, title)) = crate::visual_artifact_open::execute_artifact_action(&ctx, &request) {
                                                tab_state.write().open(id, title);
                                            }
                                        },
                                        "Edit"
                                    }
                                }
                            }
                            div { class: "notes-scroll",
                                div { class: "markdown-preview", dangerous_inner_html: "{html}" }
                            }
                        }
                    }
                }
            }
            crate::visual_artifact_surface::VisualArtifactSurface::ExcalidrawEditor {
                source_path,
            } => rsx! {
                crate::components::TabBar {}
                div {
                    style: "display:flex;flex-direction:column;flex:1;overflow:hidden;padding:0;min-height:0;height:100%;",
                    crate::views::ExcalidrawView { key: "{source_path.display()}", path: source_path }
                }
            },
            crate::visual_artifact_surface::VisualArtifactSurface::Flow { source_path } => {
                rsx! {
                    div {
                        key: "flow-{source_path.display()}",
                        style: "display:flex;flex-direction:column;flex:1;overflow:hidden;min-height:0;height:100%;",
                        crate::components::TabBar {}
                        div {
                            style: "display:flex;flex-direction:column;flex:1;overflow:hidden;padding:0;min-height:0;",
                            crate::views::FlowView { path: source_path }
                        }
                    }
                }
            }
            crate::visual_artifact_surface::VisualArtifactSurface::DesignBoard { source_path } => {
                rsx! {
                    div {
                        key: "design-board-{source_path.display()}",
                        style: "display:flex;flex-direction:column;flex:1;overflow:hidden;min-height:0;height:100%;",
                        crate::components::TabBar {}
                        div {
                            style: "display:flex;flex-direction:column;flex:1;overflow:hidden;padding:0;min-height:0;",
                            crate::views::DesignBoardView { path: source_path }
                        }
                    }
                }
            }
        };
    }

    // Clear drawing mode flag — but ONLY if it was set. Dioxus signals
    // notify subscribers on every `set`, even when the value didn't
    // change. The CM6 init effect subscribes to is_drawing_mode, so an
    // unconditional `set(false)` here fires that effect twice per
    // tab-switch (once for the cm6_load_source write, once for this
    // no-op is_drawing toggle) — wasted work and log noise.
    if *is_drawing.peek() {
        is_drawing.set(false);
    }

    // edit_body is seeded by the use_effect that watches rendered,
    // and synced from CM6 on mode switch. No eager write here.

    if crate::views::excalidraw::is_excalidraw(&rel_path) && *inspector_open.peek() {
        *inspector_open.write() = false;
    }
    if crate::views::flow::is_flow(&rel_path) && *inspector_open.peek() {
        *inspector_open.write() = false;
    }

    let title = title.clone();
    let _body = body.clone();
    let path = rel_path.clone();

    let mut renaming = use_signal(|| false);
    let mut rename_input = use_signal(|| title.clone());

    // Watch for rename trigger from sidebar context menu
    let rename_trigger = use_context::<Signal<crate::state::RenameTrigger>>();
    let mut last_rename_ver = use_signal(|| 0u64);
    if rename_trigger.read().0 > *last_rename_ver.peek() {
        *last_rename_ver.write() = rename_trigger.read().0;
        *rename_input.write() = title.clone();
        *renaming.write() = true;
    }
    let mut rename_msg: Signal<Option<String>> = use_signal(|| None);
    let path_for_rename = path.clone();
    let ctx_rename = ctx.clone();
    let history_modal_path = path.clone();
    let history_select_path = path.clone();
    let history_restore_path = path.clone();
    let publication_edit_path = path.clone();

    rsx! {
            crate::components::TabBar {}
            div { class: "notes-workspace",
            div { class: "notes-pane",
                // Conflict resolution banner
                if *conflict_detected.read() {
                    div { class: "conflict-banner",
                        span { class: "conflict-icon", "\u{26A0}" }
                        span { "This file has merge conflicts." }
                        div { class: "conflict-actions",
                            button {
                                class: "btn btn-sm btn-ghost",
                                onclick: move |_| {
                                    let content = edit_body.read().clone();
                                    let resolved = flynt_core::conflict::resolve_ours(&content);
                                    *edit_body.write() = resolved.clone();
                                    // Auto-save
                                    let p = rendered.read().as_ref().and_then(|r| r.as_ref().map(|t| t.0.clone()));
                                    let c = ctx.clone();
                                    if let Some(path) = p {
                                        spawn(async move {
                                            let project = c.project();
                                            let _ = project.save_document_content(&path, &resolved);
                                            *render_ver.write() += 1;
                                        });
                                    }
                                },
                                "Keep mine"
                            }
                            button {
                                class: "btn btn-sm btn-ghost",
                                onclick: move |_| {
                                    let content = edit_body.read().clone();
                                    let resolved = flynt_core::conflict::resolve_theirs(&content);
                                    *edit_body.write() = resolved.clone();
                                    let p = rendered.read().as_ref().and_then(|r| r.as_ref().map(|t| t.0.clone()));
                                    let c = ctx.clone();
                                    if let Some(path) = p {
                                        spawn(async move {
                                            let project = c.project();
                                            let _ = project.save_document_content(&path, &resolved);
                                            *render_ver.write() += 1;
                                        });
                                    }
                                },
                                "Keep theirs"
                            }
                            button {
                                class: "btn btn-sm btn-primary",
                                onclick: move |_| {
                                    *mode.write() = EditMode::Source;
                                },
                                "Edit manually"
                            }
                        }
                    }
                }
                div { class: "notes-topbar",
                    if *renaming.read() {
                        div { class: "rename-inline",
                            input {
                                autofocus: true,
                                class: "rename-input",
                                value: "{rename_input}",
                                oninput: move |e| *rename_input.write() = e.value(),
                                onkeydown: move |e| {
                                    if e.key() == Key::Escape {
                                        *renaming.write() = false;
                                    }
                                    if e.key() == Key::Enter {
                                        let new_title = rename_input.read().trim().to_string();
                                        if new_title.is_empty() || new_title == title { *renaming.write() = false; return; }
                                        let p = path_for_rename.clone();
                                        let c = ctx_rename.clone();
                                        spawn(async move {
                                            let project = c.project();
                                            match tokio::task::spawn_blocking(move || {
                                                project.rename_document(&p, &new_title)
                                            }).await {
                                                Ok(Ok(n)) => {
                                                    *rename_msg.write() = Some(format!("Renamed, {n} link(s) updated"));
                                                    render_ver += 1;
                                                }
                                                Ok(Err(e)) => *rename_msg.write() = Some(format!("Rename failed — {e}")),
                                                Err(e) => *rename_msg.write() = Some(format!("Rename interrupted — {e}")),
                                            }
                                            *renaming.write() = false;
                                        });
                                    }
                                },
                            }
                            {
                                let title = title.clone();
                                let path_for_rename = path_for_rename.clone();
                                let ctx_rename = ctx_rename.clone();
                                rsx! { button {
                                class: "btn btn-primary btn-xs",
                                onclick: move |_| {
                                    let new_title = rename_input.read().trim().to_string();
                                    if new_title.is_empty() || new_title == title { *renaming.write() = false; return; }
                                    let p = path_for_rename.clone();
                                    let c = ctx_rename.clone();
                                    spawn(async move {
                                        let project = c.project();
                                        match tokio::task::spawn_blocking(move || {
                                            project.rename_document(&p, &new_title)
                                        }).await {
                                            Ok(Ok(n)) => {
                                                *rename_msg.write() = Some(format!("Renamed, {n} link(s) updated"));
                                                render_ver += 1;
                                            }
                                            Ok(Err(e)) => *rename_msg.write() = Some(format!("Rename failed — {e}")),
                                            Err(e) => *rename_msg.write() = Some(format!("Rename interrupted — {e}")),
                                        }
                                        *renaming.write() = false;
                                    });
                                },
                                "Save"
                            } }
                            }
                            button { class: "btn btn-ghost btn-xs", onclick: move |_| *renaming.write() = false, "Cancel" }
                        }
                    } else {
                        h1 {
                            class: "doc-title",
                            ondoubleclick: move |_| {
                                *rename_input.write() = title.clone();
                                *renaming.write() = true;
                            },
                            "{title}"
                        }
                    }
                    if let Some(ref msg) = *rename_msg.read() {
                        span { class: "rename-msg", "{msg}" }
                    }
                    div { class: "doc-dates",
                        span { "Created {format_doc_timestamp(created_at)}" }
                        span { "Modified {format_doc_timestamp(updated_at)}" }
                    }
                    div { class: "notes-actions",
                        // Save status updated via JS to avoid Dioxus re-render
                        span { class: "save-status" }
                        button {
                            class: if *inspector_open.read() { "btn btn-ghost active" } else { "btn btn-ghost" },
                            title: "Toggle note context",
                            onclick: move |_| {
                                let open = *inspector_open.read();
                                *inspector_open.write() = !open;
                            },
                            "Context"
                        }
                        button {
                            class: "btn btn-ghost",
                            title: "Open note history",
                            onclick: move |_| {
                                *history_snapshot.write() = None;
                                *history_snapshot_error.write() = None;
                                *history_restore_message.write() = None;
                                *history_open.write() = true;
                            },
                            "History"
                        }
                        match *mode.read() {
                            EditMode::Live => rsx! {
                                span { class: "mode-hint", "⌘E source" }
                                button {
                                    class: "btn btn-ghost",
                                    onclick: move |_| {
                                        spawn(async move {
                                            let mut eval = document::eval("if(window.FlyntEditor){dioxus.send(window.FlyntEditor.getDocument().content)}else{dioxus.send('')}");
                                            if let Ok(content) = eval.recv::<String>().await {
                                                if !content.is_empty() {
                                                    *edit_body.write() = content;
                                                }
                                            }
                                            *mode.write() = EditMode::Source;
                                        });
                                    },
                                    "Source"
                                }
                            },
                            EditMode::Source => rsx! {
                                button {
                                    class: "btn btn-primary",
                                    onclick: move |_| {
                                        let content = edit_body.read().clone();
                                        let p       = path.clone();
                                        let c       = ctx.clone();

                                        spawn(async move {
                                            let project = c.project();
                                            match tokio::task::spawn_blocking(move || {
                                                project.save_document_content(&p, &content)
                                            }).await {
                                                Ok(Ok(())) => {
                                                    render_ver += 1;
                                                    *save_err.write() = None;
                                                    *save_state.write() = SaveState::Saved;
                                                }
                                                Ok(Err(e)) => *save_err.write() = Some(format!("Could not save — {e}")),
                                                Err(e)     => *save_err.write() = Some(format!("Save interrupted — {e}")),
                                            }
                                        });
                                        // Stay in Source mode — operator hits
                                        // "Live" explicitly when ready to review.
                                        // Auto-flipping caused a race where CM6
                                        // re-init lagged behind the mode change,
                                        // and the subsequent Source-toggle would
                                        // adopt CM6's stale content into edit_body.
                                    },
                                    "Save"
                                }
                                button {
                                    class: "btn btn-ghost",
                                    onclick: move |_| *mode.write() = EditMode::Live,
                                    "Live"
                                }
                            },
                            EditMode::Diagram => rsx! {
                                span { class: "mode-hint", "D2 preview" }
                                button { class: "btn btn-ghost", onclick: move |_| { let next = { *diagram_zoom.read() / 1.25 }; diagram_zoom.set(next.max(0.25)); }, "−" }
                                span { class: "mode-hint", "{((*diagram_zoom.read() * 100.0).round() as i32)}%" }
                                button { class: "btn btn-ghost", onclick: move |_| { let next = { *diagram_zoom.read() * 1.25 }; diagram_zoom.set(next.min(8.0)); }, "+" }
                                button { class: "btn btn-ghost", onclick: move |_| diagram_zoom.set(1.0), "Reset" }
                                button { class: "btn btn-ghost", onclick: move |_| *mode.write() = EditMode::Source, "Source" }
                            },
                        }
                    }
                }

                // Task metadata strip — between title bar and editor body.
                // Renders only when the doc is `kind = "task"`. Pills are
                // editable inline; changes flow through the dispatcher
                // channel installed above.
                if frontmatter.kind.as_deref() == Some("task")
                    && !crate::views::excalidraw::is_excalidraw(&rel_path)
                    && !crate::views::flow::is_flow(&rel_path)
                {
                    crate::components::TaskMetadataStrip {
                        path: rel_path.clone(),
                        frontmatter: frontmatter.clone(),
                        boards: ReadSignal::<Vec<flynt_core::models::Board>>::from(boards_cache),
                        engagements: ReadSignal::<Vec<flynt_core::models::Engagement>>::from(engagements_cache),
                    }
                }

                div { class: "notes-scroll",
                // Excalidraw and .flow files get their own editor; everything
                // else goes through the markdown editor.
                {
                let check_path = rel_path.clone();
                let is_special =
                    crate::views::excalidraw::is_excalidraw(&check_path)
                    || crate::views::flow::is_flow(&check_path);
                rsx! {
                if crate::views::excalidraw::is_excalidraw(&check_path) {
                    crate::views::ExcalidrawView { key: "{rel_path.display()}", path: rel_path.clone() }
                } else if crate::views::flow::is_flow(&check_path) {
                    crate::views::FlowView { path: rel_path.clone() }
                }

                match *mode.read() {
                    EditMode::Diagram if is_d2_path(&check_path) || d2_embed_path(&edit_body.read()).is_some() => {
                        let d2_path = resolve_d2_path(&ctx.project_root(), &check_path, &edit_body.read());
                        let svg_candidates = [
                            d2_path.with_extension("svg"),
                            ctx.project_root()
                                .join("diagrams/rendered")
                                .join(d2_path.file_name().unwrap_or_default())
                                .with_extension("svg"),
                            ctx.project_root()
                                .join("diagrams/svg")
                                .join(d2_path.file_name().unwrap_or_default())
                                .with_extension("svg"),
                        ];
                        let abs = svg_candidates
                            .iter()
                            .find(|path| path.exists())
                            .cloned()
                            .unwrap_or_else(|| d2_path.with_extension("svg"));
                        let svg = std::fs::read_to_string(&abs).ok();
                        let svg_status = if abs.exists() {
                            match (std::fs::metadata(&d2_path).and_then(|m| m.modified()), std::fs::metadata(&abs).and_then(|m| m.modified())) {
                                (Ok(source_time), Ok(render_time)) if render_time >= source_time => "SVG current",
                                (Ok(_), Ok(_)) => "SVG stale",
                                _ => "SVG present",
                            }
                        } else {
                            "SVG missing"
                        };
                        let png_path = abs.with_extension("png");
                        let png_status = if png_path.exists() {
                            match (std::fs::metadata(&d2_path).and_then(|m| m.modified()), std::fs::metadata(&png_path).and_then(|m| m.modified())) {
                                (Ok(source_time), Ok(render_time)) if render_time >= source_time => "PNG current",
                                (Ok(_), Ok(_)) => "PNG stale",
                                _ => "PNG present",
                            }
                        } else {
                            "PNG missing"
                        };
                        let zoom = *diagram_zoom.read();
                        let zoom_style = format!("width: {}%;", zoom * 100.0);
                        let pan_js = r#"(function(){
                            const panes = document.querySelectorAll('.diagram-preview-pane');
                            panes.forEach((pane) => {
                                if (pane._flyntPanBound) return;
                                pane._flyntPanBound = true;
                                pane.addEventListener('mousedown', (event) => {
                                    if (event.button !== 0) return;
                                    event.preventDefault();
                                    pane.classList.add('panning');
                                    const startX = event.clientX;
                                    const startY = event.clientY;
                                    const startLeft = pane.scrollLeft;
                                    const startTop = pane.scrollTop;
                                    const move = (moveEvent) => {
                                        pane.scrollLeft = startLeft - (moveEvent.clientX - startX);
                                        pane.scrollTop = startTop - (moveEvent.clientY - startY);
                                    };
                                    const up = () => {
                                        pane.classList.remove('panning');
                                        window.removeEventListener('mousemove', move);
                                        window.removeEventListener('mouseup', up);
                                    };
                                    window.addEventListener('mousemove', move);
                                    window.addEventListener('mouseup', up);
                                });
                            });
                        })();"#;
                        rsx! {
                            { document::eval(pan_js); }
                            div { class: "diagram-preview-pane",
                                if let Some(svg) = svg {
                                    div { class: "diagram-render-badges",
                                        span { class: "diagram-render-badge", "{svg_status}" }
                                        span { class: "diagram-render-badge", "{png_status}" }
                                    }
                                    div { class: "d2-embed", style: "{zoom_style}", dangerous_inner_html: "{svg}" }
                                } else {
                                    div { class: "d2-embed-placeholder", "D2 render pending or unavailable for {d2_path.display()}" }
                                }
                            }
                        }
                    },
                    EditMode::Diagram => rsx! {},
                    EditMode::Live if !is_special => {
                        rsx! {
                            div {
                                id: "flynt-cm-editor",
                                class: "cm-editor-container",
                            }
                        }
                    },
                    EditMode::Live => rsx! {},
                    // Source mode is disabled for special files — the markdown
                    // editor would rewrite a .flow JSON body (or .excalidraw
                    // scene) as plain text on save, corrupting the file. The
                    // mode toggle is still rendered (`EditMode::Source` is the
                    // user's stated intent) but the source-editor body
                    // short-circuits to empty for these kinds.
                    EditMode::Source if is_special => rsx! {},
                    EditMode::Source => {
                        let path_save = rel_path.clone();
                        rsx! {
                            { document::eval(r#"(function(){
                            const ed=document.getElementById('flynt-editor');
                            const pr=document.getElementById('flynt-preview');
                            if(typeof hljs!=='undefined') pr&&pr.querySelectorAll('pre code:not([data-highlighted])').forEach(b=>hljs.highlightElement(b));
                            if(!ed||!pr||ed._flynt_bound)return;
                            ed._flynt_bound=true;
                            let busy=false;
                            ed.addEventListener('scroll',function(){if(busy)return;busy=true;const p=ed.scrollTop/Math.max(1,ed.scrollHeight-ed.clientHeight);pr.scrollTop=p*(pr.scrollHeight-pr.clientHeight);requestAnimationFrame(()=>busy=false);});
                            pr.addEventListener('scroll',function(){if(busy)return;busy=true;const p=pr.scrollTop/Math.max(1,pr.scrollHeight-pr.clientHeight);ed.scrollTop=p*(ed.scrollHeight-ed.clientHeight);requestAnimationFrame(()=>busy=false);});
                        })();"#); }
                            div { class: "editor-split",
                                div { class: "editor-pane",
                                    textarea {
                                        id: "flynt-editor",
                                        class: "editor-textarea",
                                        value: "{edit_body}",
                                        oninput: move |e| *edit_body.write() = e.value(),
                                        onkeydown: move |e| {
                                            let save_key = e.modifiers().meta() || e.modifiers().ctrl();
                                            if save_key && e.key() == Key::Character("s".to_string()) {
                                                let content = edit_body.read().clone();
                                                let p       = path_save.clone();
                                                let c       = ctx_save2.clone();

                                                spawn(async move {
                                                    let project = c.project();
                                                    match tokio::task::spawn_blocking(move || {
                                                        project.save_document_content(&p, &content)
                                                    }).await {
                                                        Ok(Ok(())) => {
                                                            render_ver += 1;
                                                            *save_err.write() = None;
                                                            *save_state.write() = SaveState::Saved;
                                                        }
                                                        Ok(Err(e)) => *save_err.write() = Some(format!("Could not save — {e}")),
                                                        Err(e)     => *save_err.write() = Some(format!("Save interrupted — {e}")),
                                                    }
                                                });
                                                // ⌘S stays in Source mode; the
                                                // operator clicks Live explicitly.
                                            }
                                        },
                                    }
                                }
                                div { class: "editor-divider" }
                                div {
                                    id: "flynt-preview",
                                    class: "preview-pane",
                                    div {
                                        class: "markdown-body",
                                        // Source mode renders directly from edit_body
                                        // so the preview tracks keystrokes live. The
                                        // cached `rendered_html` is for Live mode's
                                        // post-save HTML — using it here would mean
                                        // the preview lags behind by a save cycle and
                                        // operators see no update while typing.
                                        dangerous_inner_html: "{render_html(&edit_body.read())}",
                                    }
                                }
                            }
                        }
                    },
                }
                } // rsx block
                } // check_path scope
                } // notes-scroll
            }
            if *inspector_open.read() {
                NoteInspector {
                    tab: inspector_tab,
                    body: edit_body.read().clone(),
                    frontmatter: frontmatter.clone(),
                    link_context: link_context.read().clone().flatten(),
                    on_close: move |_| *inspector_open.write() = false,
                    on_open_doc: move |doc: DocumentMeta| {
                        tab_state.write().open(doc.id.clone(), doc.title.clone());
                    },
                    on_jump_line: move |line: usize| {
                        let js = format!(
                            r#"(function(){{
                            if(window.FlyntEditor){{
                                const result = window.FlyntEditor.revealLine({line});
                                if(result && result.ok) return;
                            }}
                            const ed = document.getElementById('flynt-editor');
                            if(ed){{
                                const lines = ed.value.split('\n');
                                let pos = 0;
                                for(let i = 0; i < Math.max(0, {line} - 1) && i < lines.length; i++) pos += lines[i].length + 1;
                                ed.focus();
                                ed.setSelectionRange(pos, pos);
                                ed.scrollTop = Math.max(0, ({line} - 1) * 24);
                            }}
                        }})();"#
                        );
                        document::eval(&js);
                    },
                    on_publication_change: move |edit: PublicationEdit| {
                        let c = ctx.clone();
                        let p = publication_edit_path.clone();
                        let publication = apply_publication_edit(frontmatter.publication.clone(), edit);
                        spawn(async move {
                            let project = c.project();
                            let result = tokio::task::spawn_blocking(move || {
                                project.set_publication_config(&p, &publication)
                            })
                            .await;
                            match result {
                                Ok(Ok(())) => *render_ver.write() += 1,
                                Ok(Err(e)) => tracing::warn!("Publication update failed: {e}"),
                                Err(e) => tracing::warn!("Publication update interrupted: {e}"),
                            }
                        });
                    },
                    on_publish_preview: move |_| {
                        *publish_preview_open.write() = true;
                        *publish_preview_state.write() = None;
                        *publish_preview_error.write() = None;
                        let c = ctx.clone();
                        spawn(async move {
                            let project = c.project();
                            let result = tokio::task::spawn_blocking(move || {
                                crate::bootstrap::OmegonRuntimeContext::export_publication_preview_report(&project)
                            })
                            .await;
                            match result {
                                Ok(Ok((output_path, report))) => {
                                    *publish_preview_state.write() = Some(PublishPreviewState {
                                        output_path,
                                        exported: report.exported,
                                        skipped_private: report.skipped_private,
                                        errors: report.errors,
                                    });
                                }
                                Ok(Err(e)) => *publish_preview_error.write() = Some(format!("Publication export failed: {e}")),
                                Err(e) => *publish_preview_error.write() = Some(format!("Publication export interrupted: {e}")),
                            }
                        });
                    },
                }
            }
            if *history_open.read() {
                NoteHistoryModal {
                    path: history_modal_path.clone(),
                    state: history_state.read().clone().flatten(),
                    snapshot: history_snapshot.read().clone(),
                    current_body: edit_body.read().clone(),
                    snapshot_error: history_snapshot_error.read().clone(),
                    restore_message: history_restore_message.read().clone(),
                    on_close: move |_| *history_open.write() = false,
                    on_select_commit: move |commit: String| {
                        *history_snapshot.write() = None;
                        *history_snapshot_error.write() = None;
                        *history_restore_message.write() = None;
                        let c = ctx.clone();
                        let p = history_select_path.clone();
                        spawn(async move {
                            let project = c.project();
                            let (remote, branch) = match &project.config.sync {
                                flynt_core::models::SyncConfig::Git { remote, branch, .. } => {
                                    (remote.clone(), branch.clone())
                                }
                                _ => ("origin".into(), "main".into()),
                            };
                            let result = tokio::task::spawn_blocking(move || {
                                let git = GitSync::new(project.root.clone(), remote, branch);
                                git.read_file_at_commit(&p, &commit)
                            })
                            .await;
                            match result {
                                Ok(Ok(snapshot)) => *history_snapshot.write() = Some(snapshot),
                                Ok(Err(e)) => *history_snapshot_error.write() = Some(format!("Could not load snapshot: {e}")),
                                Err(e) => *history_snapshot_error.write() = Some(format!("Snapshot load interrupted: {e}")),
                            }
                        });
                    },
                    on_restore_snapshot: move |snapshot: FileSnapshot| {
                        *history_restore_message.write() = None;
                        let c = ctx.clone();
                        let original_path = history_restore_path.clone();
                        spawn(async move {
                            let project = c.project();
                            let short = snapshot.commit.chars().take(7).collect::<String>();
                            let stem = original_path
                                .file_stem()
                                .and_then(|s| s.to_str())
                                .unwrap_or("note");
                            let recovered = std::path::PathBuf::from("Recovered")
                                .join(format!("{stem} {short}.md"));
                            let save_result =
                                project.create_document_source(&recovered, &snapshot.content);
                            match save_result {
                                Ok(()) => {
                                    let _ = project.reindex();
                                    *render_ver.write() += 1;
                                    if let Ok(Some(meta)) = project
                                        .store
                                        .get_document_by_path(&recovered)
                                        .map(|doc| doc.map(|doc| DocumentMeta {
                                            id: doc.id,
                                            path: doc.path,
                                            title: doc.title,
                                            tags: doc.frontmatter.tags,
                                            metadata: Default::default(),
                                            entity_kind: doc.entity.map(|entity| entity.kind),
                                            updated_at: doc.updated_at,
                                        }))
                                    {
                                        tab_state.write().open(meta.id.clone(), meta.title.clone());
                                    }
                                    *history_restore_message.write() = Some(format!("Restored copy to {}", recovered.display()));
                                }
                                Err(e) => *history_snapshot_error.write() = Some(format!("Restore failed: {e}")),
                            }
                        });
                    },
                }
            }
            if *publish_preview_open.read() {
                PublishPreviewModal {
                    state: publish_preview_state.read().clone(),
                    error: publish_preview_error.read().clone(),
                    on_close: move |_| *publish_preview_open.write() = false,
                }
            }
            if let Some(state) = hover_preview.read().clone() {
                FloatingNotePreview {
                    preview: state.preview,
                    x: state.x,
                    y: state.y,
                }
            }
        }
    }
}
