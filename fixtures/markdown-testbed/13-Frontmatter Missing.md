# No Frontmatter At All

This note has no frontmatter block whatsoever — the file starts directly with
a Markdown heading. The parser's `split_frontmatter` should fall through to
its default case and treat the entire file as body.

Title, tags, and any other metadata should fall back to whatever the store
derives from the filename/content instead of frontmatter fields.
