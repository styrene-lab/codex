---
title: Links and References
tags: [testbed, links]
---

# Links and References

## Inline links

[Flynt on the web](https://example.com/flynt)

[Link with a title](https://example.com "Hover title text")

[Relative link to another note](07-Wikilinks%20and%20Embeds.md)

[Link with parentheses in the URL](<https://example.com/path_(with_parens)>)

## Reference-style links

Here is a [reference link][ref-1] and another [reference link][ref-2].

Here is a [collapsed reference][] and a [shortcut reference].

[ref-1]: https://example.com/one "Optional title one"
[ref-2]: https://example.com/two
[collapsed reference]: https://example.com/collapsed
[shortcut reference]: https://example.com/shortcut

## Autolinks

<https://example.com/autolink>

<mailto:someone@example.com>

Bare URL with no angle brackets (GFM autolink extension): https://example.com/bare-gfm-autolink

Bare email (GFM autolink extension): someone@example.com

## Images

Local image that exists:

![A small red square](assets/image.png)

Local SVG image:

![A small circular icon](assets/icon.svg)

Broken/missing local image:

![This file does not exist](assets/does-not-exist.png)

Image with a title attribute:

![Alt text](assets/image.png "Optional image title")

## Footnotes

Here is a claim that needs a citation.[^1] Here is another one.[^long-note]

[^1]: A short footnote.
[^long-note]: A longer footnote with **formatting**, a [link](https://example.com), and
    a second indented line that continues the same footnote.

## External reference badges (bare autolinked URLs)

GitHub issue: https://github.com/styrene-lab/flynt/issues/123

GitHub pull request: https://github.com/styrene-lab/flynt/pull/456

GitHub discussion: https://github.com/styrene-lab/flynt/discussions/7

GitLab issue: https://gitlab.com/example-org/example-repo/-/issues/123

GitLab merge request: https://gitlab.com/example-org/example-repo/-/merge_requests/456

Linear issue: https://linear.app/example-org/issue/ABC-123

Notion page: https://www.notion.so/Example-Page-Title-abc123def456abc123def456abc123de

Jira issue: https://example-org.atlassian.net/browse/PROJ-123

Azure DevOps work item: https://dev.azure.com/example-org/example-project/_workitems/edit/12345

Forgejo issue (explicitly tagged): https://git.example.com/example-org/example-repo/issues/1?forge=forgejo

Generic URL (no known provider, should stay a plain link, no badge): https://example.com/some/generic/page

## Same links but masked (should NOT badge — text differs from href)

[a GitHub issue, but masked](https://github.com/styrene-lab/flynt/issues/123)
