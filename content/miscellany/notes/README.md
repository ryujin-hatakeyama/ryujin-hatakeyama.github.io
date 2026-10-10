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
- `lang` is the language of this file's text: `en`, `ja`, `de`, or `mixed`. The title takes that language too; give `titleLang` (any language tag) when it differs, for example for a `mixed` note with a Japanese title.
- `description` is optional. Without it, the index shows only the title.
- `order` is optional. Notes with an `order` come first, lowest first, then the rest by slug. There is no date.
- `math: true` renders `$…$` and `$$…$$` with KaTeX at build time.
- `draft` defaults to `true`. Set `draft: false` to publish.

## Writing in another language

To add a separately written text in another language to the same note, create a companion file, conventionally `<slug>.<lang>.md`, that names the note with `of`:

```markdown
---
of: stable-slug
lang: en
title: Optional title of this text
math: false
---

Text in English. It need not translate the other text or follow its structure.
```

A companion has only `of`, `lang`, `title`, and `math`; everything else belongs to the note's main file. Each language may have one passage per note. All passages appear on the note's one page, English first, then Japanese, then German, then any `mixed` passage. A passage title is shown as a heading, so that passage's own headings start at `###`.

So a Japanese-only note is one file with `lang: ja`; adding English later is a new `<slug>.en.md` beside it.

The body supports headings from `##` down (the title is the page's only `#`), paragraphs, lists, block quotations, inline code, fenced code blocks, tables, and footnotes. Raw HTML is rejected, and links must use HTTP(S), `mailto:`, a root-relative path, or a fragment.
