---
title: Horizontal Rules
tags: [testbed, hr]
---

# Horizontal Rules

Paragraph before a rule.

---

Paragraph between rules.

***

Paragraph between rules.

___

Paragraph after the last rule.

## Rule immediately after a heading, no blank line

## Heading right above
---

## Spaced-out markers

- - -

* * *

_ _ _

## Rule vs. list ambiguity

A single `---` right under a paragraph with no blank line can be parsed as a setext H2 instead of a rule:
text right above
---

A `***` or `___` is never ambiguous with setext headings, only `-` is:
text right above
***
