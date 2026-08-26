---
title: Lists
tags: [testbed, lists]
---

# Lists

## Unordered — tight

- Item one
- Item two
- Item three

## Unordered — loose (blank lines between items)

- Item one, with a longer paragraph of text to see how wrapping and loose spacing interact.

- Item two.

- Item three.

## Mixed markers (still one list per CommonMark, but starting a new bullet char starts a new list)

- Dash item
* Asterisk item
+ Plus item

## Ordered — default start

1. First
2. Second
3. Third

## Ordered — custom start number

5. Five
6. Six
7. Seven

## Ordered — all same number (should still increment visually)

1. One
1. One again
1. One again

## Nested lists (5 levels deep)

- Level 1
  - Level 2
    - Level 3
      - Level 4
        - Level 5

## Nested ordered inside unordered inside ordered

1. Top ordered item
   - Nested unordered
     1. Nested ordered
        - Nested unordered again
   - Back to nested unordered

## Task lists

- [ ] Unchecked task
- [x] Checked task
- [X] Checked task (uppercase X)
- [ ] Unchecked task with **bold** and a [[03-Lists|wikilink]]
- [x] Checked task with ~~strikethrough~~

### Nested task lists

- [ ] Parent task
  - [x] Completed subtask
  - [ ] Pending subtask
    - [ ] Deeply nested subtask

## List interrupted by a paragraph, then continued

- Item one
- Item two

A plain paragraph in between.

- Item three (CommonMark: is this the same list or a new one?)

## List item containing a code block

- Item with a fenced code block:

  ```rust
  fn main() {}
  ```

- Item with an indented code block:

      indented code inside a list item

## List item containing a blockquote

- Item with a quote:

  > A blockquote nested inside a list item.

## Empty list items

-
- Item after an empty one
-

## Very long single item (wrapping stress)

- This is a single long list item meant to stress-test text wrapping behavior inside a bullet: the quick brown fox jumps over the lazy dog, repeatedly, again and again, until the line is long enough to wrap at least twice in a normally sized editor pane, and then wrap once more just to be sure the hanging indent lines up with the bullet marker above it.
