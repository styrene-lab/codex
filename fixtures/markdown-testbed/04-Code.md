---
title: Code
tags: [testbed, code]
---

# Code

## Inline code

Use `let x = 5;` to bind a variable. A span with a backtick inside: `` `nested` ``.

Empty inline code: `` (should this even render?)

## Fenced code blocks — common languages

```rust
pub fn resolve_document(document: &DocumentReference, store: &dyn ProjectStore) -> Result<Document, ResolutionError> {
    match document {
        DocumentReference::Id(id) => store.get_document(&DocumentId(*id))?.ok_or(ResolutionError::DocumentNotFound),
        DocumentReference::RelativePath(path) => store.get_document_by_path(Path::new(path))?.ok_or(ResolutionError::DocumentNotFound),
    }
}
```

```typescript
interface Note {
  id: string;
  title: string;
  tags: string[];
}

const note: Note = { id: "1", title: "Test", tags: ["a", "b"] };
```

```python
def fibonacci(n: int) -> int:
    a, b = 0, 1
    for _ in range(n):
        a, b = b, a + b
    return a
```

```bash
#!/usr/bin/env bash
set -euo pipefail
for f in *.md; do
  echo "processing: $f"
done
```

```json
{
  "name": "flynt",
  "version": "0.13.0",
  "tags": ["notes", "kb"],
  "nested": { "a": [1, 2, 3], "b": null, "c": true }
}
```

```toml
[appearance]
theme = "alpharius"
font_size = "medium"
```

```yaml
title: Example
tags:
  - one
  - two
nested:
  key: value
```

```sql
SELECT id, title, updated_at
FROM documents
WHERE tags @> ARRAY['engineering']
ORDER BY updated_at DESC
LIMIT 10;
```

```diff
- old line removed
+ new line added
  unchanged context line
```

```html
<div class="admonition admonition-tip">
  <div class="admonition-title">Tip</div>
</div>
```

```css
.admonition-tip {
  border-left: 4px solid var(--accent);
  padding: 0.5rem 1rem;
}
```

```
Plain fenced block with no language hint at all.
Should render as a generic code block, no syntax highlighting.
```

```unknownlang
This uses a language hint the highlighter has never heard of.
Should fail gracefully, not crash.
```

## Indented code blocks (four-space rule)

    fn indented_example() {
        println!("no fence, just four-space indentation");
    }

## Fence edge cases

~~~
Tilde-fenced code block instead of backticks.
~~~

````
Four-backtick fence, useful when the code itself contains ``` triple backticks.
```rust
this line looks like a fence open but is just content
```
````

```rust
// A code fence containing a line that looks like it could close early: ```
fn still_one_block() {}
```
