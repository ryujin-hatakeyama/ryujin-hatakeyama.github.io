# Diary

Each `*.md` file here (other than this README) is one diary entry. All published entries appear in full on `/miscellany/diary/`, newest first.

```markdown
---
date: 2026-10-10
lang: ja
title: Optional title
slug: optional-slug
math: false
draft: true
---

Body in Markdown. Entries may be one line or many paragraphs.
```

- `date` is the date of the entry, written by hand. It is never taken from Git or the build.
- `lang` is the language of this file's text: `en`, `ja`, `de`, or `mixed`. Give `titleLang` when the title's language differs.
- `title` is optional; the date always heads the entry.
- Each entry is linkable at `/miscellany/diary/#diary-YYYY-MM-DD`. If two entries share a date, give at least one a `slug`; its anchor becomes `#diary-YYYY-MM-DD-<slug>`. Changing `date` or `slug` changes the link.
- `draft` defaults to `true`. Set `draft: false` to publish.

To add a separately written text in another language to the same entry, create a companion file, conventionally `<date>.<lang>.md`, that names the entry by its identity, its date and any slug (for example `2026-10-10` or `2026-10-10-evening`):

```markdown
---
of: 2026-10-10
lang: en
---

Text in English.
```

Entries are associated only through `of`; two entries on the same date stay separate. A companion has only `of`, `lang`, `title`, and `math`, and each language may have one passage per entry.

Because entries share one page, bodies use headings from `###` down (`####` in a passage with its own title) and cannot contain footnotes. Raw HTML is rejected, and links follow the same rules as Notes.
