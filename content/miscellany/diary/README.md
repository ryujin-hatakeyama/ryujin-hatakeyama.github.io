# Diary

Each `*.md` file here (other than this README) is one diary entry. Entries are stored one per file and shown by month: `/miscellany/diary/` lists the months that have published entries (newest first), and each month's page, `/miscellany/diary/<year>/<month>/` (for example `/miscellany/diary/2026/10/`), shows that month's entries in full, oldest first. A month without published entries has no page.

```markdown
---
date: 2026-10-10
lang: ja
title: Optional title
slug: optional-slug
published: 2026-10-12
math: false
draft: true
---

Body in Markdown. Entries may be one line or many paragraphs.
```

- `date` is the date of the entry, written by hand. It is never taken from Git or the build.
- `lang` is the language of this file's text: `en`, `ja`, `de`, or `mixed`. Give `titleLang` when the title's language differs.
- `title` is optional; the date always heads the entry.
- `published` is optional: the day the entry was published, which is a different thing from `date`, the day it records. With it, the entry is announced in Updates on that day (see `../README.md`); without it, it is not.
- Each entry is linkable at `/miscellany/diary/YYYY/MM/#diary-YYYY-MM-DD`. If two entries share a date, give at least one a `slug`; its anchor becomes `#diary-YYYY-MM-DD-<slug>`. Changing `date` or `slug` changes the link. Links in the earlier form, `/miscellany/diary/#diary-…`, are forwarded to the month's page by the Diary page's script.
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

Because entries share their month's page, bodies use headings from `###` down (`####` in a passage with its own title) and cannot contain footnotes. Raw HTML is rejected, and links follow the same rules as Notes.
