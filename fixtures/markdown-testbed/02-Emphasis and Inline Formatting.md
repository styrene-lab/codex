---
title: Emphasis and Inline Formatting
tags: [testbed, emphasis]
---

# Emphasis and Inline Formatting

## Basic

*italic with asterisks* and _italic with underscores_

**bold with asterisks** and __bold with underscores__

***bold italic with asterisks*** and ___bold italic with underscores___

~~strikethrough~~

## Nesting

**bold with *nested italic* inside**

*italic with **nested bold** inside*

**bold with ~~nested strikethrough~~ inside**

~~strikethrough with **nested bold**~~

## Mid-word emphasis (CommonMark flanking rules)

Mid*word*emphasis should render as italic (asterisks allow mid-word).

Mid_word_emphasis should NOT render as italic (underscores forbid mid-word by default).

Snake_case_variable_name should stay literal.

5*3*2 is not emphasis (numbers, no word boundary intent).

## Unmatched / dangling markers

This has one unmatched *asterisk with no closer.

This has one unmatched **bold opener with no closer.

Random ** in the middle of a sentence with no pair.

## Escaping

\*not italic\* — escaped asterisks stay literal.

\*\*not bold\*\* — escaped double asterisks stay literal.

Literal backslash: \\ and a literal asterisk: \*

## Inline code interacting with emphasis

`code spans are never *emphasized* inside backticks`

**bold containing `inline code`**

A code span with a literal backtick: `` `backtick` ``

A code span with leading/trailing spaces: ` code `

## Hard line breaks

Line one with two trailing spaces  
Line two follows on a new line.

Line one with a trailing backslash\
Line two follows on a new line.

Line one with no break marker
Line two is just a soft-wrapped continuation.
