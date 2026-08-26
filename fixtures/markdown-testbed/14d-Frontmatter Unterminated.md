---
title: Unterminated
tags: [testbed]

# Unterminated Frontmatter Block

The file above opens with `---` but never closes with a matching `---` line
anywhere in the file. Per `split_frontmatter`, this means the YAML branch
never matches at all — the entire file, including the leading `---`, should
be treated as plain body content (the leading `---` renders as a horizontal
rule, and the "frontmatter-looking" lines above render as literal text/list
items, not as metadata).
