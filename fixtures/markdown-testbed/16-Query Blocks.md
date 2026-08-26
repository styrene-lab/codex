---
title: Query Blocks
tags: [testbed, queries]
---

# Query Blocks

Inline query engine blocks: fenced code with the `query` language hint, only
executed in the reading-view render path (not the live CM6 editor).

## TABLE — default columns

```query
TABLE
```

## TABLE — explicit columns

```query
TABLE title, tags, updated_at
```

## TABLE — filtered by tag, sorted, limited

```query
TABLE title, tags
WHERE tags CONTAINS "testbed"
SORT title DESC
LIMIT 5
```

## TABLE — filtered by title substring

```query
TABLE title
WHERE title CONTAINS "Frontmatter"
```

## LIST — single field

```query
LIST title
```

## TASK — all tasks

```query
TASK
```

## TASK — filtered by status

```query
TASK
WHERE status = "todo"
NOT ARCHIVED
```

## TASK — filtered by priority

```query
TASK
WHERE priority = "high"
```

## Malformed query (unknown query type — should render an inline error, not crash)

```query
FROBNICATE everything
```

## Empty query block

```query
```
