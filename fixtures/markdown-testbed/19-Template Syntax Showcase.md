---
title: Template Syntax Showcase
tags: [testbed, templates]
---

# Template Syntax Showcase

Flynt's daily-note/template variables are `{{title}}`, `{{date}}`, `{{time}}`,
`{{year}}`, `{{month}}`, `{{day}}`, `{{weekday}}`, and `{{project}}`. Unlike a
real template file, this note is a plain saved document — the braces below
are literal text, not live substitutions. Useful for checking that the live
and reading views don't mistake `{{...}}` for something else (an embed, a
query fence, etc.) when it shows up as ordinary body content.

Literal, unexpanded template markers:

```
title = "{{title}}"

# {{title}}

**Date:** {{date}} at {{time}}
**Project:** {{project}}
**Weekday:** {{weekday}}, {{year}}-{{month}}-{{day}}
```

Same markers inline, not fenced: {{title}} {{date}} {{project}}

A brace pair that is not a template variable: {{ this has spaces }} and {not double braces} and {{{triple braces}}}.
