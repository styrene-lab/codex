+++
title = "Frontmatter TOML"
tags = ["testbed", "frontmatter", "toml"]

[data]
kind = "reference"
priority = "medium"
nested_array = [1, 2, 3]
nested_table = { a = "x", b = "y" }
+++

# TOML Frontmatter

This note uses `+++`-delimited TOML frontmatter (as used internally by `.flow`/`.board`
wrapper files and Styrene design docs). The body should render normally; the
frontmatter block itself should never leak into the rendered output.

A body that happens to contain a line that looks like a TOML table header:

```toml
[not_actually_frontmatter]
key = "this is inside a code fence, not a second frontmatter block"
```
