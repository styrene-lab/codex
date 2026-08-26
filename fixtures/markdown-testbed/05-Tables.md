---
title: Tables
tags: [testbed, tables]
---

# Tables

## Basic GFM table

| Name  | Role      | Active |
| ----- | --------- | ------ |
| Alice | Engineer  | true   |
| Bob   | Designer  | false  |
| Carol | Manager   | true   |

## Column alignment

| Left | Center | Right |
| :--- | :----: | ----: |
| a    |   b    |     c |
| long left value | c | 1234 |

## Table without leading/trailing pipes

Name | Score
--- | ---
Alice | 91
Bob | 84

## Ragged rows (fewer/more cells than header)

| A | B | C |
| - | - | - |
| 1 | 2 |
| 3 | 4 | 5 | 6 |

## Empty cells

| Column A | Column B |
| -------- | -------- |
|          | filled   |
| filled   |          |
|          |          |

## Cells with inline formatting, code, and links

| Feature | Status | Notes |
| ------- | ------ | ----- |
| **Bold header value** | `code` | see [[05-Tables\|this note]] |
| *Italic* and ~~strike~~ | ✅ | [external](https://example.com) |

## Cell containing a pipe (escaped)

| Expression | Result |
| ---------- | ------ |
| `a \| b`   | bitwise or |
| literal escaped pipe: \| | still one cell |

## Wide table (many columns)

| A | B | C | D | E | F | G | H | I | J |
| - | - | - | - | - | - | - | - | - | - |
| 1 | 2 | 3 | 4 | 5 | 6 | 7 | 8 | 9 | 10 |

## Table immediately followed by a paragraph (no blank line)

| X | Y |
| - | - |
| 1 | 2 |
This paragraph starts right after the table with no blank line separator.
