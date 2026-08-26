---
title: Blockquotes
tags: [testbed, blockquotes]
---

# Blockquotes

## Simple

> A single-line blockquote.

## Multi-line, single quote block

> Line one of the quote.
> Line two of the quote, still part of the same blockquote.

## Lazy continuation (no leading `>` on the second line)

> Line one of the quote.
Line two with no leading marker — CommonMark treats this as a lazy continuation of the same quote.

## Multi-paragraph

> First paragraph.
>
> Second paragraph, same blockquote.

## Nested blockquotes

> Level 1
>> Level 2
>>> Level 3
>>> Level 3 continuation
>>Level 2 continuation
>Level 1 continuation

## Blockquote containing a list

> - Item one
> - Item two
>   - Nested item

## Blockquote containing a code block

> ```rust
> fn quoted_code() {}
> ```

## Blockquote containing a heading

> ## A heading inside a blockquote

## Blockquote with inline formatting and a wikilink

> This quote has **bold**, *italic*, `code`, and a [[01-Headings|wikilink]].

## Empty blockquote

>

## Blockquote immediately followed by a paragraph, no blank line

> The quote ends here.
This paragraph starts right after with no blank line separator.
