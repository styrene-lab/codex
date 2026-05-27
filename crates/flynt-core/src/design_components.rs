//! Static Design Board component registry.
//!
//! Component cells are semantic inputs rendered to ordinary HTML/CSS/JS before
//! they enter the existing iframe `srcdoc` pipeline. This keeps Design Board
//! cells portable and isolated while giving agents a patchable component layer.

use anyhow::{anyhow, bail, Context};
use serde_json::{json, Value};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RenderedCell {
    pub html: String,
    pub css: String,
    pub js: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DesignComponentMetadata {
    pub name: &'static str,
    pub category: &'static str,
    pub description: &'static str,
    pub variants: &'static [&'static str],
    pub props_schema: Value,
    pub examples: Vec<Value>,
    pub rendering_constraints: &'static [&'static str],
}

pub struct DesignComponentDefinition {
    pub name: &'static str,
    pub category: &'static str,
    pub description: &'static str,
    pub variants: &'static [&'static str],
    pub props_schema: fn() -> Value,
    pub examples: fn() -> Vec<Value>,
    pub rendering_constraints: &'static [&'static str],
    pub render: fn(&Value, Option<&str>) -> anyhow::Result<RenderedCell>,
}

impl DesignComponentDefinition {
    pub fn metadata(&self) -> DesignComponentMetadata {
        DesignComponentMetadata {
            name: self.name,
            category: self.category,
            description: self.description,
            variants: self.variants,
            props_schema: (self.props_schema)(),
            examples: (self.examples)(),
            rendering_constraints: self.rendering_constraints,
        }
    }

    pub fn validate_variant<'a>(&self, variant: Option<&'a str>) -> anyhow::Result<Option<&'a str>> {
        let Some(variant) = variant else {
            return Ok(None);
        };
        if self.variants.contains(&variant) {
            Ok(Some(variant))
        } else {
            bail!(
                "unknown variant '{variant}' for component '{}'; expected one of: {}",
                self.name,
                self.variants.join(", ")
            )
        }
    }
}

pub fn list_components() -> Vec<DesignComponentMetadata> {
    registry().iter().map(|definition| definition.metadata()).collect()
}

pub fn get_component(name: &str) -> Option<&'static DesignComponentDefinition> {
    registry()
        .iter()
        .find(|definition| definition.name.eq_ignore_ascii_case(name))
}

pub fn render_component(
    name: &str,
    props: &Value,
    variant: Option<&str>,
) -> anyhow::Result<RenderedCell> {
    let definition = get_component(name).ok_or_else(|| {
        anyhow!(
            "unknown Design Board component '{name}'; available components: {}",
            registry()
                .iter()
                .map(|definition| definition.name)
                .collect::<Vec<_>>()
                .join(", ")
        )
    })?;
    let variant = definition.validate_variant(variant)?;
    (definition.render)(props, variant)
        .with_context(|| format!("render Design Board component '{}'", definition.name))
}

fn registry() -> &'static [DesignComponentDefinition] {
    &[PANEL]
}

const COMPONENT_CONSTRAINTS: &[&str] = &[
    "outermost element must fill the cell with h-full",
    "use theme tokens instead of hard-coded colors",
    "avoid Tailwind arbitrary-value classes",
    "escape all string props before rendering",
];

const PANEL: DesignComponentDefinition = DesignComponentDefinition {
    name: "Panel",
    category: "layout",
    description: "Generic shadcn-style card shell with optional title, description, badge, body, and footer.",
    variants: &["default", "muted", "accent"],
    props_schema: panel_props_schema,
    examples: panel_examples,
    rendering_constraints: COMPONENT_CONSTRAINTS,
    render: render_panel,
};

fn panel_props_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            "title": { "type": "string" },
            "description": { "type": "string" },
            "badge": { "type": "string" },
            "body": { "type": "string" },
            "footer": { "type": "string" }
        }
    })
}

fn panel_examples() -> Vec<Value> {
    vec![json!({
        "component": "Panel",
        "variant": "default",
        "props": {
            "title": "Research source",
            "description": "Canonical note projection",
            "badge": "source",
            "body": "Source metadata and synthesis stay in markdown; the board renders a refreshable projection.",
            "footer": "Open source note"
        }
    })]
}

fn render_panel(props: &Value, variant: Option<&str>) -> anyhow::Result<RenderedCell> {
    let variant = variant.unwrap_or("default");
    let title = prop_string(props, "title")?;
    let description = prop_string(props, "description")?;
    let badge = prop_string(props, "badge")?;
    let body = prop_string(props, "body")?;
    let footer = prop_string(props, "footer")?;

    let surface_classes = match variant {
        "default" => "border-border bg-card text-card-foreground",
        "muted" => "border-border bg-muted text-foreground",
        "accent" => "border-primary bg-card text-card-foreground",
        other => bail!("unknown Panel variant '{other}'"),
    };

    let header = if title.is_some() || description.is_some() || badge.is_some() {
        format!(
            "<div class=\"flex items-start justify-between gap-3\"><div class=\"min-w-0 space-y-1\">{}{}{}</div>{}</div>",
            title
                .as_deref()
                .map(|value| format!("<h3 class=\"text-lg font-semibold tracking-tight text-foreground\">{}</h3>", escape_html(value)))
                .unwrap_or_default(),
            description
                .as_deref()
                .map(|value| format!("<p class=\"text-sm text-muted-foreground\">{}</p>", escape_html(value)))
                .unwrap_or_default(),
            if title.is_none() && description.is_none() { "<span></span>" } else { "" },
            badge
                .as_deref()
                .map(|value| format!("<span class=\"shrink-0 rounded-full border border-border px-2 py-1 text-xs font-medium text-muted-foreground\">{}</span>", escape_html(value)))
                .unwrap_or_default(),
        )
    } else {
        String::new()
    };

    let body = body
        .as_deref()
        .map(|value| format!("<p class=\"text-sm leading-6 text-muted-foreground\">{}</p>", escape_html(value)))
        .unwrap_or_default();
    let footer = footer
        .as_deref()
        .map(|value| format!("<div class=\"mt-auto border-t border-border pt-3 text-xs font-medium text-primary\">{}</div>", escape_html(value)))
        .unwrap_or_default();

    Ok(RenderedCell {
        html: format!(
            "<article class=\"h-full rounded-lg border {surface_classes} p-5 shadow-sm\"><div class=\"flex h-full flex-col gap-4\">{header}{body}{footer}</div></article>"
        ),
        css: String::new(),
        js: None,
    })
}

fn prop_string(props: &Value, key: &str) -> anyhow::Result<Option<String>> {
    match props.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => Ok(Some(value.clone())),
        Some(_) => bail!("property '{key}' must be a string"),
    }
}

fn escape_html(input: &str) -> String {
    input
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lists_panel_metadata() {
        let components = list_components();
        let panel = components.iter().find(|component| component.name == "Panel").unwrap();
        assert_eq!(panel.category, "layout");
        assert!(panel.variants.contains(&"default"));
        assert!(!panel.examples.is_empty());
    }

    #[test]
    fn panel_renders_h_full_shell() {
        let rendered = render_component(
            "Panel",
            &json!({ "title": "Hello", "body": "World" }),
            Some("default"),
        )
        .unwrap();
        assert!(rendered.html.contains("h-full"));
        assert!(rendered.html.contains("Hello"));
        assert_eq!(rendered.css, "");
        assert_eq!(rendered.js, None);
    }

    #[test]
    fn panel_escapes_string_props() {
        let rendered = render_component(
            "Panel",
            &json!({ "title": "<script>alert('x')</script>" }),
            None,
        )
        .unwrap();
        assert!(rendered.html.contains("&lt;script&gt;"));
        assert!(!rendered.html.contains("<script>alert"));
    }

    #[test]
    fn rejects_unknown_component() {
        let err = render_component("Nope", &json!({}), None).unwrap_err().to_string();
        assert!(err.contains("unknown Design Board component"));
    }

    #[test]
    fn rejects_unknown_variant() {
        let err = render_component("Panel", &json!({}), Some("loud"))
            .unwrap_err()
            .to_string();
        assert!(err.contains("unknown variant 'loud'"));
    }

    #[test]
    fn rejects_non_string_panel_props() {
        let err = render_component("Panel", &json!({ "title": 42 }), None)
            .unwrap_err()
            .to_string();
        assert!(err.contains("property 'title' must be a string") || err.contains("render Design Board component 'Panel'"));
    }
}
