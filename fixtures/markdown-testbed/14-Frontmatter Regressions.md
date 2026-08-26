---
title: Frontmatter Regressions
tags: [testbed, frontmatter, regressions]
---
---

The line directly above this paragraph is the very first line of the body,
and it is itself a bare `---` (a horizontal rule). This is the known
regression shape behind "body-only save received a second frontmatter
block": a body that begins with something that looks like a frontmatter
delimiter.

**Manual regression check:** open this note, make a trivial body edit below
this paragraph, and save. The save must succeed as a body-only edit — it
must not be misread as introducing a second frontmatter block, and the
leading `---` above must survive as ordinary body content (a horizontal
rule), not be swallowed or duplicated.

---

A second `---` further down the body, just to see whether position (first
line vs. mid-body) changes the behavior.

## Sibling files for the remaining frontmatter edge cases

See:

- [[14a-Frontmatter Malformed TOML]] — invalid TOML inside `+++` delimiters
- [[14b-Frontmatter Malformed YAML]] — invalid YAML inside `---` delimiters
- [[14c-Frontmatter Empty Block]] — an empty frontmatter block
- [[14d-Frontmatter Unterminated]] — a frontmatter opener with no closing delimiter
