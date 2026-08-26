---
title: Malformed YAML
tags: [testbed
  broken: indentation
    that: does not close properly
---

# Malformed YAML Frontmatter

The frontmatter block above is deliberately invalid YAML (unterminated flow
sequence, inconsistent indentation). This should also fall back to
`Frontmatter::default()` without panicking.
