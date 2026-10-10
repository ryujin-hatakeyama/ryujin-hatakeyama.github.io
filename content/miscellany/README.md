# Miscellany

Source for `/miscellany/` and its three sections. Nothing here is announced in Updates or the RSS feed.

| Section | Source | Route |
|---|---|---|
| Reading | `reading.yaml` | `/miscellany/reading/` |
| Notes | `notes/*.md` | `/miscellany/notes/` and `/miscellany/notes/<slug>/` |
| Diary | `diary/*.md` | `/miscellany/diary/` |

Everything committed to this repository is public, including records marked `draft: true`. A draft flag only keeps a record off the generated site; it is not a privacy mechanism. Keep private material out of the repository.

## Adding a reading item

Append an entry to `items` in `reading.yaml`. Items appear in the order written; nothing is sorted, rated, or dated.

```yaml
items:
  - author: Author name as published
    title: Original title, as written
    lang: ja                       # optional: language of the title and author's name
    url: https://example.org/work  # optional: an external reference (HTTP or HTTPS)
    note: A short personal note.   # optional: plain text
    noteLang: ja                   # optional: language of the note, if not English
```

Only `title` is required. `author` may be omitted for anonymous works. `lang` and `noteLang` take language tags such as `ja`, `de`, or `zh-Hant`; omit them for English. Keep original titles and names; do not translate them. Unknown fields are rejected.

Notes and diary entries are documented in `notes/README.md` and `diary/README.md`.
