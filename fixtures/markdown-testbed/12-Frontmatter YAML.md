---
title: Frontmatter YAML
tags: [testbed, frontmatter, yaml]
aliases: [yaml-fm-test]
date: 2026-08-26
custom_number: 42
custom_bool: true
custom_list:
  - one
  - two
  - three
custom_nested:
  key: value
  another_key: another value
---

# YAML Frontmatter

This note uses `---`-delimited YAML frontmatter, the more common convention for
plain notes. Round-tripping should preserve every field verbatim, including
nested structures, on every save.

A body line that contains a literal string with a colon, which YAML parsers
sometimes mis-tokenize if frontmatter boundaries are detected carelessly: `key: value` written as inline text, not frontmatter.
