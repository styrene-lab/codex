---
title: Admonitions
tags: [testbed, admonitions]
---

# Admonitions

GitHub/Obsidian-style callouts: `> [!KIND]` on its own line, followed by blockquote-continuation lines.

## All six kinds

> [!NOTE]
> This is a note admonition with a single line of body text.

> [!TIP]
> This is a tip admonition.

> [!IMPORTANT]
> This is an important admonition.

> [!WARNING]
> This is a warning admonition.

> [!CAUTION]
> This is a caution admonition.

> [!DANGER]
> This is a danger admonition.

## Case variants (should be case-insensitive)

> [!note]
> Lowercase kind marker.

> [!Note]
> Title-case kind marker.

## Multi-paragraph body

> [!NOTE]
> First paragraph of the note.
>
> Second paragraph of the same note, after a blank blockquote line.

## Body with inline formatting

> [!WARNING]
> This warning has **bold**, *italic*, `inline code`, and a [[01-Headings|wikilink]].

## Body with a list

> [!TIP]
> Steps to follow:
> 1. First step
> 2. Second step
> - Bullet variant too

## Admonition immediately followed by a plain blockquote (boundary test)

> [!NOTE]
> The admonition body.

> A plain blockquote right after, with no `[!KIND]` marker — should render as a normal quote, not an admonition.

## Plain blockquote that merely mentions admonition syntax as text

> Someone wrote the literal text [!NOTE] in the middle of a sentence, not at the start of the quote.

## Unknown/invalid kind marker (should not match any of the six)

> [!UNKNOWN]
> This kind is not in the recognized set.
