+++
id = "rtl-note-direction-support"
kind = "design_node"
title = "Direction-aware rendering for primarily-RTL notes"
status = "seed"
tags = ["i18n", "rtl", "editor", "markdown", "speculative"]

[data]
issue_type = "feature"
priority = 4
trigger = "an operator writes a note that is primarily Arabic/Hebrew/other-RTL prose, not just RTL text embedded in an LTR document"
+++

# Direction-aware rendering for primarily-RTL notes

## Status

**Seed.** Captured while reviewing the markdown testbed's Unicode/RTL fixture
(`fixtures/markdown-testbed/17-Unicode and Whitespace.md`). Not scoped or
committed to — just recorded so it isn't lost.

## What's true today

Neither the CM6 Live editor nor the comrak/Source-mode preview set `dir` on
paragraphs. Both inherit the page's default `direction: ltr`, so every block
renders left-aligned regardless of content. This is correct for the testbed's
actual content — labeled multilingual examples like "Arabic: `<phrase>`" in
an English-structured list, where the paragraph's dominant structure really
is LTR even though it contains an RTL phrase. The Unicode Bidi Algorithm
still handles character/word-level reordering correctly within the RTL runs
regardless of block alignment, confirmed visually (Arabic/Hebrew
sentence-final punctuation lands at the visual left edge, as it should for
RTL text) — this isn't a rendering bug.

## The gap

A note that is *genuinely, predominantly* RTL prose — not a mixed-language
example, an actual Arabic or Hebrew document — would also render left-
aligned today, because nothing auto-detects or honors a primarily-RTL
direction. Editors built for RTL-inclusive use (Notion, Word, Obsidian with
the RTL plugin) typically auto-detect dominant direction per block and
right-align accordingly, usually via HTML's `dir="auto"` or CSS
`unicode-bidi: plaintext`.

## Why this is a seed, not a deferred design

Unlike the flow-editor phase deferrals in this directory, this hasn't been
scoped enough to say what "why we're holding off" actually means yet — there's
no known operator need today, and the right fix (auto-detect per paragraph?
per document? an explicit note-level setting?) isn't decided. Revisit if:

- An operator actually writes primarily-RTL notes and finds left-alignment
  wrong for them, or
- Flynt gains a real internationalization push and this becomes part of a
  broader i18n audit rather than a one-off.

## Rough shape, if picked up

Not a plan — just where the pieces would likely live:

- CM6 Live mode: per-line or per-block `dir="auto"` equivalent, likely via a
  CodeMirror line-attribute decoration keyed on the line's dominant script
  (Unicode bidi class of the first strong character).
- Comrak/Source-mode preview: `unicode-bidi: plaintext` (or `dir="auto"`) on
  `.markdown-body` paragraph/heading/list-item elements, letting the browser's
  native per-element auto-detection do the work rather than hand-rolling
  script detection in Rust.
- Test coverage: a dedicated testbed file that is *itself* written primarily
  in Arabic or Hebrew (not English-structured examples containing RTL
  phrases) — `17-Unicode and Whitespace.md` doesn't exercise this case.
