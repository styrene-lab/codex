---
title: HTML and Escaping
tags: [testbed, html, escaping]
---

# HTML and Escaping

## Raw inline HTML

Text with <strong>raw HTML bold</strong> and <em>raw HTML italic</em> mixed with **Markdown bold**.

A raw <span style="color: red;">styled span</span> inline.

## Raw HTML blocks

<div class="raw-html-block">
  <p>A raw HTML block with nested tags.</p>
</div>

<details>
<summary>Raw HTML details/summary (native disclosure widget)</summary>

Body content inside a details block, which may itself be Markdown depending
on the renderer's HTML-block handling rules.

</details>

## HTML comments

<!-- This is an HTML comment and should not render as visible text. -->

Visible text after a comment.

## HTML entities

Named entities: &amp; &lt; &gt; &quot; &copy; &mdash; &hellip;

Numeric entities: &#65; &#8364; &#x1F600;

## Character escaping

Backslash-escaped punctuation: \! \" \# \$ \% \& \' \( \) \* \+ \, \- \. \/ \: \; \< \= \> \? \@ \[ \\ \] \^ \_ \` \{ \| \} \~

A literal angle bracket that is not a tag: 5 < 10 and 10 > 5

An ampersand not part of an entity: Q&A session, R&D team

## Potentially unsafe HTML (renderer runs with unsafe HTML enabled)

<script>console.log("should this execute or be shown as inert text?");</script>

<img src="x" onerror="alert(1)" alt="broken image with an event handler attribute">

## Mixed HTML and Markdown in the same block

<div>

**This bold text is Markdown inside an HTML block** — blank-line-separated HTML blocks allow Markdown inside them.

</div>
