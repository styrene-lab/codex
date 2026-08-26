---
title: Wikilinks and Embeds
tags: [testbed, wikilinks, embeds]
---

# Wikilinks and Embeds

## Plain wikilinks

A simple wikilink to [[01-Headings]].

A wikilink with a display alias: [[01-Headings|Jump to Headings]].

A wikilink with an anchor: [[01-Headings#H2 Heading]].

A wikilink with both alias and anchor: [[01-Headings#H2 Heading|see this heading]].

## Self-referencing wikilink

This note links to itself: [[07-Wikilinks and Embeds]].

## Broken / dangling wikilink

This points nowhere: [[This Note Does Not Exist]].

## Wikilink vs. standard markdown link to the same target

Standard link form: [Headings](01-Headings.md)

Wikilink form: [[01-Headings]]

## Wikilinks packed together

See [[01-Headings]], [[02-Emphasis and Inline Formatting]], and [[03-Lists]] for related material.

## Wikilink inside other formatting

**Bold with a [[01-Headings|wikilink]] inside.**

- List item with a [[03-Lists|wikilink]]
- [ ] Task with a [[03-Lists|wikilink]]

> Blockquote with a [[01-Headings|wikilink]] inside.

## Excalidraw drawing embed

Bare embed (resolved via the `drawings/` fallback path):

![[drawing.excalidraw]]

Embed with an explicit width:

![[drawing.excalidraw|300]]

## D2 diagram embed

![[diagram.d2]]

## Image embeds via wikilink-embed syntax

![[image.png]]

![[icon.svg]]

## Broken embeds

Missing drawing:

![[does-not-exist.excalidraw]]

Missing diagram:

![[does-not-exist.d2]]

Missing image:

![[does-not-exist.png]]

## Flow and design-board wrappers (opened as separate documents, not inline embeds)

These are not rendered inline — they resolve to their own canonical surface when opened:

- [[Release Flow]] — a `.flow` wrapper note
- [[Launch Dashboard]] — a `.board` wrapper note
