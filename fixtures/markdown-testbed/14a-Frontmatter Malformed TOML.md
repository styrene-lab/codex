+++
title = "Malformed TOML"
tags = [testbed, unquoted, invalid syntax
+++

# Malformed TOML Frontmatter

The frontmatter block above is deliberately invalid TOML (unterminated array,
unquoted bareword elements). `split_frontmatter` should fall back to
`Frontmatter::default()` via `unwrap_or_default()` rather than panicking —
this file existing and opening without a crash is the actual test.
