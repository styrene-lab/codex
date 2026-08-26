---
title: Unicode and Whitespace
tags: [testbed, unicode, whitespace]
---

# Unicode and Whitespace

## Emoji

Simple emoji: 🎉 🚀 ✅ ❌ 🔥 📝

Emoji with skin-tone modifier: 👍🏽

Combined/ZWJ emoji sequence: 👨‍👩‍👧‍👦 (family, zero-width joiners)

Emoji inside a heading: ## 🚀 Launch Checklist (rendered as body text here, not an actual heading, to test emoji-in-heading elsewhere — see the real one below)

## 🚀 An Actual Heading With a Leading Emoji

## CJK text

Chinese: 你好，世界。这是一个测试段落，用来检查中文文本的换行和排版。

Japanese: こんにちは世界。これはマークダウンのレンダリングをテストするための文章です。

Korean: 안녕하세요 세계. 이것은 마크다운 렌더링을 테스트하기 위한 문장입니다.

## Right-to-left text

Arabic: مرحبا بالعالم. هذا نص تجريبي لاختبار عرض النصوص من اليمين إلى اليسار.

Hebrew: שלום עולם. זהו טקסט לדוגמה לבדיקת תמיכה בטקסט מימין לשמאל.

Mixed LTR/RTL in one line: The Arabic word for "hello" is مرحبا and it reads right-to-left.

## Combining diacritics and normalization edge cases

Composed: café, naïve, Zürich

Decomposed (combining marks): cafe´, nai¨ve (may render identically to the composed forms above depending on normalization)

Combining marks stacked: ë̸̢̛̗̼̳́̈ (zalgo-adjacent stress case)

## Zero-width and invisible characters

Zero-width space between these⁠words. Zero-width joiner between these‍words. A byte-order-mark-adjacent case: ﻿ at the start of a line is not rendered here directly but worth testing with an actual BOM-prefixed file.

## Tabs vs. spaces

A line with a	tab	character	between	words.

A code block mixing tabs and spaces for indentation:

```
	tab-indented line
    space-indented line
```

## Trailing whitespace hard breaks (visible markers below, invisible above)

Two trailing spaces after this line create a hard break.  
Backslash after this line creates a hard break.\
No marker after this line is just a soft wrap.

## Extremely long unbroken line (word-wrap stress)

Supercalifragilisticexpialidocioussupercalifragilisticexpialidocioussupercalifragilisticexpialidocioussupercalifragilisticexpialidocioussupercalifragilisticexpialidocious

## Extremely long line with normal spaces (line-wrap stress)

The quick brown fox jumps over the lazy dog the quick brown fox jumps over the lazy dog the quick brown fox jumps over the lazy dog the quick brown fox jumps over the lazy dog the quick brown fox jumps over the lazy dog the quick brown fox jumps over the lazy dog

## Non-breaking spaces

A non-breaking space here:  between these two words (U+00A0), which should not wrap at that point.
