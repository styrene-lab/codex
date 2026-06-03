//! Static Design Board component registry.
//!
//! Component cells are semantic inputs rendered to ordinary HTML/CSS/JS before
//! they enter the existing iframe `srcdoc` pipeline. This keeps Design Board
//! cells portable and isolated while giving agents a patchable component layer.

use anyhow::{Context, anyhow, bail};
use serde_json::{Value, json};

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

    pub fn validate_variant<'a>(
        &self,
        variant: Option<&'a str>,
    ) -> anyhow::Result<Option<&'a str>> {
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
    registry()
        .iter()
        .map(|definition| definition.metadata())
        .collect()
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
    &[
        PANEL,
        FRAME,
        TEXT_BLOCK,
        COLUMNS,
        STACK,
        BUTTON_ROW,
        IMAGE_PLACEHOLDER,
    ]
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

const FRAME: DesignComponentDefinition = DesignComponentDefinition {
    name: "Frame",
    category: "layout",
    description: "Generic visual region for page sections, artboards, brochure panels, and whiteboard zones.",
    variants: &["plain", "card", "bordered", "hero", "muted", "accent"],
    props_schema: frame_props_schema,
    examples: frame_examples,
    rendering_constraints: COMPONENT_CONSTRAINTS,
    render: render_frame,
};

const TEXT_BLOCK: DesignComponentDefinition = DesignComponentDefinition {
    name: "TextBlock",
    category: "typography",
    description: "Structured typography block for headings, body copy, quotes, captions, and fine print.",
    variants: &["body", "heading", "lead", "quote", "caption", "fine-print"],
    props_schema: text_block_props_schema,
    examples: text_block_examples,
    rendering_constraints: COMPONENT_CONSTRAINTS,
    render: render_text_block,
};

const COLUMNS: DesignComponentDefinition = DesignComponentDefinition {
    name: "Columns",
    category: "layout",
    description: "Multi-column content layout for brochures, resumes, comparisons, and web sections.",
    variants: &["two", "three", "asymmetric-left", "asymmetric-right"],
    props_schema: columns_props_schema,
    examples: columns_examples,
    rendering_constraints: COMPONENT_CONSTRAINTS,
    render: render_columns,
};

const STACK: DesignComponentDefinition = DesignComponentDefinition {
    name: "Stack",
    category: "layout",
    description: "Vertical list/stack of simple structured items with optional bullets, checks, or numbering.",
    variants: &["default", "compact", "bullets", "checklist", "numbered"],
    props_schema: stack_props_schema,
    examples: stack_examples,
    rendering_constraints: COMPONENT_CONSTRAINTS,
    render: render_stack,
};

const BUTTON_ROW: DesignComponentDefinition = DesignComponentDefinition {
    name: "ButtonRow",
    category: "actions",
    description: "Primary/secondary action row for website mockups, product one-pagers, and document CTAs.",
    variants: &["left", "center", "right", "stacked"],
    props_schema: button_row_props_schema,
    examples: button_row_examples,
    rendering_constraints: COMPONENT_CONSTRAINTS,
    render: render_button_row,
};

const IMAGE_PLACEHOLDER: DesignComponentDefinition = DesignComponentDefinition {
    name: "ImagePlaceholder",
    category: "media",
    description: "Theme-aware media/screenshot placeholder with aspect and caption support.",
    variants: &["default", "browser", "device", "plain"],
    props_schema: image_placeholder_props_schema,
    examples: image_placeholder_examples,
    rendering_constraints: COMPONENT_CONSTRAINTS,
    render: render_image_placeholder,
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
        .map(|value| {
            format!(
                "<p class=\"text-sm leading-6 text-muted-foreground\">{}</p>",
                escape_html(value)
            )
        })
        .unwrap_or_default();
    let footer = footer
        .as_deref()
        .map(|value| format!("<div class=\"mt-auto border-t border-border pt-3 text-xs font-medium text-primary\">{}</div>", escape_html(value)))
        .unwrap_or_default();

    Ok(RenderedCell {
        html: format!(
            "<article data-flynt-focus-kind=\"component\" data-flynt-component=\"Panel\" data-flynt-component-part=\"root\" class=\"h-full rounded-lg border {surface_classes} p-5 shadow-sm\"><div class=\"flex h-full flex-col gap-4\">{header}{body}{footer}</div></article>"
        ),
        css: String::new(),
        js: None,
    })
}

fn frame_props_schema() -> Value {
    json!({"type":"object","additionalProperties":false,"properties":{"title":{"type":"string"},"subtitle":{"type":"string"},"body":{"type":"string"},"padding":{"type":"string","enum":["sm","md","lg"]}}})
}
fn frame_examples() -> Vec<Value> {
    vec![
        json!({"component":"Frame","variant":"card","props":{"title":"Hero section","subtitle":"Above the fold","body":"Use as a flexible artboard region.","padding":"lg"}}),
    ]
}
fn render_frame(props: &Value, variant: Option<&str>) -> anyhow::Result<RenderedCell> {
    let title = prop_string(props, "title")?;
    let subtitle = prop_string(props, "subtitle")?;
    let body = prop_string(props, "body")?;
    let padding = prop_string(props, "padding")?.unwrap_or_else(|| "md".into());
    let pad = match padding.as_str() {
        "sm" => "p-3",
        "md" => "p-5",
        "lg" => "p-8",
        other => bail!("unknown padding '{other}'"),
    };
    let classes = match variant.unwrap_or("card") {
        "plain" => "bg-transparent text-foreground",
        "card" => "rounded-lg border border-border bg-card text-card-foreground shadow-sm",
        "bordered" => "rounded-lg border border-border bg-transparent text-foreground",
        "hero" => "rounded-xl border border-primary bg-card text-card-foreground shadow-sm",
        "muted" => "rounded-lg border border-border bg-muted text-foreground",
        "accent" => "rounded-lg border border-primary bg-card text-card-foreground",
        other => bail!("unknown Frame variant '{other}'"),
    };
    Ok(RenderedCell {
        html: format!(
            "<section data-flynt-focus-kind=\"component\" data-flynt-component=\"Frame\" data-flynt-component-part=\"root\" class=\"h-full {classes} {pad}\"><div class=\"flex h-full flex-col justify-center gap-3\">{}{}{}</div></section>",
            opt_h(
                "h2",
                "text-2xl font-bold tracking-tight text-foreground",
                title
            ),
            opt_p("text-sm text-muted-foreground", subtitle),
            opt_p("text-sm leading-6 text-muted-foreground", body)
        ),
        css: String::new(),
        js: None,
    })
}

fn text_block_props_schema() -> Value {
    json!({"type":"object","additionalProperties":false,"properties":{"eyebrow":{"type":"string"},"heading":{"type":"string"},"body":{"type":"string"},"align":{"type":"string","enum":["left","center","right"]}}})
}
fn text_block_examples() -> Vec<Value> {
    vec![
        json!({"component":"TextBlock","variant":"lead","props":{"eyebrow":"About","heading":"Senior systems engineer","body":"I design local-first tools for source-backed work.","align":"left"}}),
    ]
}
fn render_text_block(props: &Value, variant: Option<&str>) -> anyhow::Result<RenderedCell> {
    let eyebrow = prop_string(props, "eyebrow")?;
    let heading = prop_string(props, "heading")?;
    let body = prop_string(props, "body")?;
    let align = prop_string(props, "align")?.unwrap_or_else(|| "left".into());
    let align_class = match align.as_str() {
        "left" => "text-left items-start",
        "center" => "text-center items-center",
        "right" => "text-right items-end",
        other => bail!("unknown align '{other}'"),
    };
    let (hcls, bcls) = match variant.unwrap_or("body") {
        "body" => (
            "text-xl font-semibold tracking-tight text-foreground",
            "text-sm leading-6 text-muted-foreground",
        ),
        "heading" => (
            "text-3xl font-bold tracking-tight text-foreground",
            "text-base leading-7 text-muted-foreground",
        ),
        "lead" => (
            "text-2xl font-bold tracking-tight text-foreground",
            "text-lg leading-7 text-muted-foreground",
        ),
        "quote" => (
            "text-2xl font-semibold italic tracking-tight text-foreground",
            "text-sm text-muted-foreground",
        ),
        "caption" => (
            "text-sm font-semibold text-foreground",
            "text-xs text-muted-foreground",
        ),
        "fine-print" => (
            "text-sm font-medium text-foreground",
            "text-xs leading-5 text-muted-foreground",
        ),
        other => bail!("unknown TextBlock variant '{other}'"),
    };
    Ok(RenderedCell {
        html: format!(
            "<section data-flynt-focus-kind=\"component\" data-flynt-component=\"TextBlock\" data-flynt-component-part=\"root\" class=\"h-full flex flex-col justify-center gap-3 {align_class}\">{}{}{}</section>",
            opt_p(
                "text-xs font-semibold uppercase tracking-wide text-primary",
                eyebrow
            ),
            opt_h("h2", hcls, heading),
            opt_p(bcls, body)
        ),
        css: String::new(),
        js: None,
    })
}

fn columns_props_schema() -> Value {
    json!({"type":"object","additionalProperties":false,"properties":{"columns":{"type":"array","items":{"type":"object","properties":{"title":{"type":"string"},"body":{"type":"string"}},"required":["title"]}}}})
}
fn columns_examples() -> Vec<Value> {
    vec![
        json!({"component":"Columns","variant":"two","props":{"columns":[{"title":"Problem","body":"Research fragments across tools."},{"title":"Solution","body":"Compose structured visual projections."}]}}),
    ]
}
fn render_columns(props: &Value, variant: Option<&str>) -> anyhow::Result<RenderedCell> {
    let cols = props
        .get("columns")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("property 'columns' must be an array"))?;
    let grid = match variant.unwrap_or("two") {
        "two" => "grid-cols-2",
        "three" => "grid-cols-3",
        "asymmetric-left" => "grid-cols-3",
        "asymmetric-right" => "grid-cols-3",
        other => bail!("unknown Columns variant '{other}'"),
    };
    let mut out = String::new();
    for (i, c) in cols.iter().enumerate() {
        let title = value_string(c, "title")?;
        let body = optional_value_string(c, "body")?;
        let span = match (variant.unwrap_or("two"), i) {
            ("asymmetric-left", 0) => " md:col-span-2",
            ("asymmetric-right", 1) => " md:col-span-2",
            _ => "",
        };
        out.push_str(&format!("<div class=\"rounded-lg border border-border bg-card p-4{span}\"><h3 class=\"text-sm font-semibold text-foreground\">{}</h3>{}</div>", escape_html(&title), body.map(|b|format!("<p class=\"mt-2 text-sm leading-6 text-muted-foreground\">{}</p>",escape_html(&b))).unwrap_or_default()));
    }
    Ok(RenderedCell {
        html: format!(
            "<div data-flynt-focus-kind=\"component\" data-flynt-component=\"Columns\" data-flynt-component-part=\"root\" class=\"h-full grid {grid} gap-4\">{out}</div>"
        ),
        css: String::new(),
        js: None,
    })
}

fn stack_props_schema() -> Value {
    json!({"type":"object","additionalProperties":false,"properties":{"title":{"type":"string"},"items":{"type":"array","items":{"type":"string"}}}})
}
fn stack_examples() -> Vec<Value> {
    vec![
        json!({"component":"Stack","variant":"checklist","props":{"title":"Launch checklist","items":["Frame primitive","TextBlock primitive","Columns primitive"]}}),
    ]
}
fn render_stack(props: &Value, variant: Option<&str>) -> anyhow::Result<RenderedCell> {
    let title = prop_string(props, "title")?;
    let items = props
        .get("items")
        .and_then(Value::as_array)
        .ok_or_else(|| anyhow!("property 'items' must be an array"))?;
    let variant = variant.unwrap_or("default");
    let mut lis = String::new();
    for (idx, item) in items.iter().enumerate() {
        let text = item
            .as_str()
            .ok_or_else(|| anyhow!("stack items must be strings"))?;
        let marker = match variant {
            "checklist" => "✓".into(),
            "numbered" => format!("{}.", idx + 1),
            "bullets" => "•".into(),
            "default" | "compact" => "".into(),
            other => bail!("unknown Stack variant '{other}'"),
        };
        lis.push_str(&format!("<li class=\"flex gap-3 text-sm text-muted-foreground\"><span class=\"w-5 shrink-0 text-primary\">{}</span><span>{}</span></li>",marker,escape_html(text)));
    }
    Ok(RenderedCell {
        html: format!(
            "<section data-flynt-focus-kind=\"component\" data-flynt-component=\"Stack\" data-flynt-component-part=\"root\" class=\"h-full rounded-lg border border-border bg-card p-5\">{}<ul class=\"flex h-full flex-col gap-3\">{lis}</ul></section>",
            opt_h("h3", "mb-3 text-sm font-semibold text-foreground", title)
        ),
        css: String::new(),
        js: None,
    })
}

fn button_row_props_schema() -> Value {
    json!({"type":"object","additionalProperties":false,"properties":{"primary":{"type":"string"},"secondary":{"type":"string"},"tertiary":{"type":"string"}}})
}
fn button_row_examples() -> Vec<Value> {
    vec![
        json!({"component":"ButtonRow","variant":"left","props":{"primary":"Get started","secondary":"Learn more"}}),
    ]
}
fn render_button_row(props: &Value, variant: Option<&str>) -> anyhow::Result<RenderedCell> {
    let primary = prop_string(props, "primary")?;
    let secondary = prop_string(props, "secondary")?;
    let tertiary = prop_string(props, "tertiary")?;
    let layout = match variant.unwrap_or("left") {
        "left" => "items-center justify-start",
        "center" => "items-center justify-center",
        "right" => "items-center justify-end",
        "stacked" => "items-stretch justify-center flex-col",
        other => bail!("unknown ButtonRow variant '{other}'"),
    };
    let mut buttons = String::new();
    if let Some(v) = primary {
        buttons.push_str(&format!("<span class=\"inline-flex items-center justify-center rounded-md bg-primary px-4 py-2 text-sm font-medium text-primary-foreground\">{}</span>",escape_html(&v)));
    }
    if let Some(v) = secondary {
        buttons.push_str(&format!("<span class=\"inline-flex items-center justify-center rounded-md border border-border bg-card px-4 py-2 text-sm font-medium text-foreground\">{}</span>",escape_html(&v)));
    }
    if let Some(v) = tertiary {
        buttons.push_str(&format!("<span class=\"inline-flex items-center justify-center px-3 py-2 text-sm font-medium text-muted-foreground\">{}</span>",escape_html(&v)));
    }
    Ok(RenderedCell {
        html: format!(
            "<div data-flynt-focus-kind=\"component\" data-flynt-component=\"ButtonRow\" data-flynt-component-part=\"root\" class=\"h-full flex gap-3 {layout}\">{buttons}</div>"
        ),
        css: String::new(),
        js: None,
    })
}

fn image_placeholder_props_schema() -> Value {
    json!({"type":"object","additionalProperties":false,"properties":{"label":{"type":"string"},"caption":{"type":"string"},"aspect":{"type":"string"}}})
}
fn image_placeholder_examples() -> Vec<Value> {
    vec![
        json!({"component":"ImagePlaceholder","variant":"browser","props":{"label":"Product screenshot","caption":"Dashboard concept","aspect":"16:9"}}),
    ]
}
fn render_image_placeholder(props: &Value, variant: Option<&str>) -> anyhow::Result<RenderedCell> {
    let label = prop_string(props, "label")?.unwrap_or_else(|| "Image placeholder".into());
    let caption = prop_string(props, "caption")?;
    let aspect = prop_string(props, "aspect")?.unwrap_or_else(|| "16:9".into());
    let aspect_cls = match aspect.as_str() {
        "1:1" => "aspect-square",
        "4:3" => "aspect-video",
        "16:9" => "aspect-video",
        "auto" => "h-full",
        other => bail!("unknown aspect '{other}'"),
    };
    let chrome = match variant.unwrap_or("default") {
        "browser" => {
            "<div class=\"flex gap-1 border-b border-border p-2\"><span class=\"h-2 w-2 rounded-full bg-destructive\"></span><span class=\"h-2 w-2 rounded-full bg-yellow-500\"></span><span class=\"h-2 w-2 rounded-full bg-green-500\"></span></div>"
        }
        "default" | "device" | "plain" => "",
        other => bail!("unknown ImagePlaceholder variant '{other}'"),
    };
    Ok(RenderedCell {
        html: format!(
            "<figure data-flynt-focus-kind=\"component\" data-flynt-component=\"ImagePlaceholder\" data-flynt-component-part=\"root\" class=\"h-full rounded-lg border border-dashed border-border bg-muted/40 text-muted-foreground overflow-hidden\">{chrome}<div class=\"flex {aspect_cls} h-full flex-col items-center justify-center gap-2 p-4 text-center\"><div class=\"text-sm font-medium text-foreground\">{}</div>{}</div></figure>",
            escape_html(&label),
            caption
                .map(|c| format!(
                    "<figcaption class=\"text-xs\">{}</figcaption>",
                    escape_html(&c)
                ))
                .unwrap_or_default()
        ),
        css: String::new(),
        js: None,
    })
}

fn opt_h(tag: &str, class: &str, value: Option<String>) -> String {
    value
        .map(|v| format!("<{tag} class=\"{class}\">{}</{tag}>", escape_html(&v)))
        .unwrap_or_default()
}
fn opt_p(class: &str, value: Option<String>) -> String {
    value
        .map(|v| format!("<p class=\"{class}\">{}</p>", escape_html(&v)))
        .unwrap_or_default()
}
fn value_string(value: &Value, key: &str) -> anyhow::Result<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_string)
        .ok_or_else(|| anyhow!("property '{key}' must be a string"))
}
fn optional_value_string(value: &Value, key: &str) -> anyhow::Result<Option<String>> {
    match value.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(v)) => Ok(Some(v.clone())),
        Some(_) => bail!("property '{key}' must be a string"),
    }
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
    fn lists_foundation_metadata() {
        let components = list_components();
        for expected in [
            "Panel",
            "Frame",
            "TextBlock",
            "Columns",
            "Stack",
            "ButtonRow",
            "ImagePlaceholder",
        ] {
            let component = components
                .iter()
                .find(|component| component.name == expected)
                .unwrap();
            assert!(
                !component.examples.is_empty(),
                "missing example for {expected}"
            );
            assert!(
                !component.variants.is_empty(),
                "missing variants for {expected}"
            );
        }
    }

    #[test]
    fn foundation_components_render_h_full() {
        let cases = [
            ("Frame", json!({"title":"Frame"}), Some("card")),
            (
                "TextBlock",
                json!({"heading":"Heading","body":"Body"}),
                Some("lead"),
            ),
            (
                "Columns",
                json!({"columns":[{"title":"A"},{"title":"B"}]}),
                Some("two"),
            ),
            ("Stack", json!({"items":["One","Two"]}), Some("checklist")),
            (
                "ButtonRow",
                json!({"primary":"Go","secondary":"Back"}),
                Some("center"),
            ),
            (
                "ImagePlaceholder",
                json!({"label":"Shot","aspect":"16:9"}),
                Some("browser"),
            ),
        ];
        for (name, props, variant) in cases {
            let rendered = render_component(name, &props, variant).unwrap();
            assert!(rendered.html.contains("h-full"), "{name} did not fill cell");
        }
    }

    #[test]
    fn foundation_components_emit_focus_metadata() {
        let cases = [
            ("Panel", json!({"title":"Panel"}), Some("default")),
            ("Frame", json!({"title":"Frame"}), Some("card")),
            (
                "TextBlock",
                json!({"heading":"Heading","body":"Body"}),
                Some("lead"),
            ),
            (
                "Columns",
                json!({"columns":[{"title":"A"},{"title":"B"}]}),
                Some("two"),
            ),
            ("Stack", json!({"items":["One","Two"]}), Some("checklist")),
            (
                "ButtonRow",
                json!({"primary":"Go","secondary":"Back"}),
                Some("center"),
            ),
            (
                "ImagePlaceholder",
                json!({"label":"Shot","aspect":"16:9"}),
                Some("browser"),
            ),
        ];
        for (name, props, variant) in cases {
            let rendered = render_component(name, &props, variant).unwrap();
            assert!(
                rendered
                    .html
                    .contains("data-flynt-focus-kind=\"component\""),
                "{name} missing focus kind"
            );
            assert!(
                rendered
                    .html
                    .contains(&format!("data-flynt-component=\"{name}\"")),
                "{name} missing component metadata"
            );
            assert!(
                rendered.html.contains("data-flynt-component-part=\"root\""),
                "{name} missing component part"
            );
        }
    }

    #[test]
    fn foundation_components_escape_props() {
        let rendered =
            render_component("TextBlock", &json!({"heading":"<b>bad</b>"}), None).unwrap();
        assert!(rendered.html.contains("&lt;b&gt;bad&lt;/b&gt;"));
        assert!(!rendered.html.contains("<b>bad</b>"));
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
        let err = render_component("Nope", &json!({}), None)
            .unwrap_err()
            .to_string();
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
        assert!(
            err.contains("property 'title' must be a string")
                || err.contains("render Design Board component 'Panel'")
        );
    }
}
