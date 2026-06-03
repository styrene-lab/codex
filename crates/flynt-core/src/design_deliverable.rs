use regex::Regex;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DesignDeliverableSpec {
    pub title: String,
    pub subtitle: String,
    pub sections: Vec<DeliverableSection>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliverableSection {
    pub id: String,
    pub title: String,
    pub description: String,
    pub items: Vec<DeliverableItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DeliverableItem {
    pub title: String,
    pub description: String,
    pub asset_path: PathBuf,
    pub source_path: Option<PathBuf>,
}

pub fn parse_static_deliverable(html: &str) -> anyhow::Result<DesignDeliverableSpec> {
    let title = capture_first(html, r"(?is)<h1[^>]*>(.*?)</h1>")
        .map(clean_text)
        .unwrap_or_else(|| "Untitled Deliverable".into());
    let subtitle = capture_first(html, r#"(?is)<p\s+class=["']subtitle["'][^>]*>(.*?)</p>"#)
        .map(clean_text)
        .unwrap_or_default();

    let details_re = Regex::new(
        r#"(?is)<details\s+id=["']([^"']+)["'][^>]*>\s*<summary>(.*?)</summary>\s*<div\s+class=["']section-body["'][^>]*>(.*?)</div>\s*</details>"#,
    )?;
    let mut sections = Vec::new();
    for caps in details_re.captures_iter(html) {
        let id = caps
            .get(1)
            .map(|m| m.as_str())
            .unwrap_or_default()
            .to_string();
        let title = caps
            .get(2)
            .map(|m| clean_summary(m.as_str()))
            .unwrap_or_else(|| id.clone());
        let body = caps.get(3).map(|m| m.as_str()).unwrap_or_default();
        let items = parse_items(body)?;
        let description = items
            .first()
            .map(|item| item.description.clone())
            .unwrap_or_default();
        sections.push(DeliverableSection {
            id,
            title,
            description,
            items,
        });
    }

    Ok(DesignDeliverableSpec {
        title,
        subtitle,
        sections,
    })
}

fn parse_items(body: &str) -> anyhow::Result<Vec<DeliverableItem>> {
    let item_re = Regex::new(
        r#"(?is)<h3[^>]*>(.*?)</h3>\s*<p[^>]*>(.*?)</p>\s*<div\s+class=["']diagram["'][^>]*>\s*<img\s+src=["']([^"']+)["'][^>]*>.*?</div>"#,
    )?;
    let mut items = Vec::new();
    for caps in item_re.captures_iter(body) {
        let title = caps
            .get(1)
            .map(|m| clean_text(m.as_str()))
            .unwrap_or_default();
        let description = caps
            .get(2)
            .map(|m| clean_text(m.as_str()))
            .unwrap_or_default();
        let asset = caps.get(3).map(|m| m.as_str()).unwrap_or_default();
        items.push(DeliverableItem {
            title,
            description,
            asset_path: PathBuf::from(asset),
            source_path: None,
        });
    }
    Ok(items)
}

fn capture_first<'a>(text: &'a str, pattern: &str) -> Option<&'a str> {
    Regex::new(pattern)
        .ok()?
        .captures(text)?
        .get(1)
        .map(|m| m.as_str())
}

fn clean_summary(text: &str) -> String {
    let without_count = Regex::new(r#"(?is)<span\s+class=["']count["'][^>]*>.*?</span>"#)
        .map(|re| re.replace_all(text, "").to_string())
        .unwrap_or_else(|_| text.to_string());
    clean_text(&without_count)
}

fn clean_text(text: &str) -> String {
    let tags = Regex::new(r"(?is)<[^>]+>").unwrap();
    html_unescape(&tags.replace_all(text, " "))
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn html_unescape(text: &str) -> String {
    text.replace("&amp;", "&")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&lt;", "<")
        .replace("&gt;", ">")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_static_diagram_site() {
        let html = r#"
        <h1>Qrypt Cluster Architecture</h1>
        <p class="subtitle">31 diagrams · click any section to expand</p>
        <details id="overview" open>
        <summary>Architecture Overview <span class="count">3 diagrams</span></summary>
        <div class="section-body">
        <h3>v1 — Single manager</h3>
        <p>One active manager runs everything.</p>
        <div class="diagram"><img src="overview-v1.svg"><div class="filename">overview-v1</div></div>
        </div>
        </details>
        "#;

        let spec = parse_static_deliverable(html).unwrap();
        assert_eq!(spec.title, "Qrypt Cluster Architecture");
        assert_eq!(spec.subtitle, "31 diagrams · click any section to expand");
        assert_eq!(spec.sections[0].id, "overview");
        assert_eq!(spec.sections[0].title, "Architecture Overview");
        assert_eq!(
            spec.sections[0].items[0].asset_path,
            PathBuf::from("overview-v1.svg")
        );
    }
}
