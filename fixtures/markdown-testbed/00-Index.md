---
title: Markdown Testbed Index
tags: [testbed, index]
---

# Markdown Testbed

A dedicated fixture vault for the Markdown viewer/rendering/preview
workstream: exhaustive edge-case coverage for screenshot-based regression
tracking, and a staging area for trying new rendering elements before they
land in the real editor.

Each linked note below isolates one concern so a rendering regression can be
traced to a specific file/section instead of hunting through general notes.

## Coverage

- [[01-Headings]] — ATX and setext headings, edge cases
- [[02-Emphasis and Inline Formatting]] — bold/italic/strikethrough, nesting, escaping, hard breaks
- [[03-Lists]] — ordered/unordered, nesting, task lists, loose/tight, interruption
- [[04-Code]] — inline code, fenced blocks across languages, fence edge cases
- [[05-Tables]] — alignment, ragged rows, empty cells, wide tables, escaped pipes
- [[06-Links and References]] — inline/reference/autolinks, images, footnotes, external provider badges
- [[07-Wikilinks and Embeds]] — `[[...]]` variants, `![[...]]` embeds (Excalidraw/D2/images), broken links
- [[08-Admonitions]] — all six GitHub/Obsidian callout kinds, boundary cases
- [[09-Blockquotes]] — nesting, lazy continuation, mixed content
- [[10-Horizontal Rules]] — all three marker styles, ambiguity with setext headings
- [[11-Frontmatter TOML]] — `+++`-delimited TOML frontmatter
- [[12-Frontmatter YAML]] — `---`-delimited YAML frontmatter
- [[13-Frontmatter Missing]] — no frontmatter block at all
- [[14-Frontmatter Regressions]] — known regression shapes, plus:
  - [[14a-Frontmatter Malformed TOML]]
  - [[14b-Frontmatter Malformed YAML]]
  - [[14c-Frontmatter Empty Block]]
  - [[14d-Frontmatter Unterminated]]
- [[15-HTML and Escaping]] — raw HTML, entities, character escaping
- [[16-Query Blocks]] — inline `TABLE`/`LIST`/`TASK` query fences
- [[17-Unicode and Whitespace]] — emoji, CJK, RTL, combining marks, tabs, hard breaks
- [[18-Large Document Stress]] — 120 headings, a 200-row table, deep nesting, a 150-line code block
- [[19-Template Syntax Showcase]] — literal `{{variable}}` markers as ordinary body text

## Non-Markdown surfaces referenced from the coverage notes

- [[Release Flow]] — `.flow` wrapper, opened via canonical surface resolution
- [[Launch Dashboard]] — `.board` wrapper, opened via canonical surface resolution
- `drawings/drawing.excalidraw` + `.svg` — embedded inline from [[07-Wikilinks and Embeds]]
- `diagrams/diagram.d2` + `.svg` — embedded inline from [[07-Wikilinks and Embeds]]
- `assets/image.png`, `assets/icon.svg` — image embeds
- `tasks/` — three sample tasks (ready/doing/done) so [[16-Query Blocks]]'s `TASK` queries return real rows

## Using this vault

```
scripts/launch-local-app.sh fixtures/markdown-testbed
```

or, for a real-data-shaped validation build:

```
scripts/launch-candidate.sh fixtures/markdown-testbed
```

Add new files here for any new rendering feature under development —
isolate it in its own note the same way the existing coverage is split, so
it becomes a permanent regression fixture once the feature ships.
