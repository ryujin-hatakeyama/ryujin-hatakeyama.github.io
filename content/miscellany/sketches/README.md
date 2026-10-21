# Sketches

Each `*.md` file here (other than this README) is a sketch, published at `/miscellany/sketches/<slug>/`.

```markdown
---
slug: stable-slug
title: Sketch title
lang: ja
date: 2026-10-12
description: Optional one-line description shown in the index.
published: 2026-10-14
math: false
draft: true
---

Body in Markdown.
```

- `slug` fixes the sketch's URL; do not change it once published. Sketch slugs are separate from note slugs.
- `date` is the date of the sketch, written by hand. It is never taken from Git or the build. The index lists sketches newest first, then by slug.
- `published` is optional: the day the sketch was published, which need not be its `date`. With it, the sketch is announced in Updates on that day (see `../README.md`).
- `lang`, `titleLang`, `description`, and `math` mean what they mean for Notes (see `notes/README.md`).
- `draft` defaults to `true`. Set `draft: false` to publish.

The body follows the rules for Notes: headings from `##` down, footnotes, mathematics, quotations, tables, and links; raw HTML is rejected. Companion files in another language name the sketch with `of: <slug>`, exactly as for Notes.
