# Notes

Each `*.md` file here (other than this README) is a study note, published at `/miscellany/notes/<slug>/`.

```markdown
---
slug: stable-slug
title: Note title
lang: en
description: Optional one-line description shown in the index.
order: 10
math: false
draft: true
---

Body in Markdown.
```

- `slug` fixes the note's URL; do not change it once published.
- `lang` is `en` or `ja` and marks the language of the whole note.
- `description` is optional. Without it, the index shows only the title.
- `order` is optional. Notes with an `order` come first, lowest first, then the rest by slug. There is no date.
- `math: true` renders `$…$` and `$$…$$` with KaTeX at build time.
- `draft` defaults to `true`. Set `draft: false` to publish.

The body supports headings from `##` down (the title is the page's only `#`), paragraphs, lists, block quotations, inline code, fenced code blocks, tables, and footnotes. Raw HTML is rejected, and links must use HTTP(S), `mailto:`, a root-relative path, or a fragment.
