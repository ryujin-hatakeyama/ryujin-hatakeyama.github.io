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

Only `title` is required. `author` may be omitted for anonymous works. `lang` describes the work: its title and author's name, in any language tag such as `ja`, `de`, or `zh-Hant`; omit it for English. Keep original titles and names; do not translate them. Unknown fields are rejected.

The commentary is your own writing about the work, and its language is independent of the work's. Give one `note`, or several separately written comments with `notes`:

```yaml
  - title: 架空の本
    lang: ja
    notes:
      en: A comment written in English.
      ja: 日本語で書いた別の感想。
```

Commentary languages are `en`, `ja`, `de`, or `mixed` (a comment that deliberately moves between languages). Each language may appear once. A work without commentary is simply listed.

## Languages

Notes, diary entries, and reading commentary may be written in several languages. Each separately authored text is a *passage* with its own language; an article with several passages is still one article, with one URL or anchor and one entry in every list. The texts need not be translations of one another and need not share structure.

When published passages use more than one language, Miscellany pages offer a small control: All, then each language used (English, 日本語, Deutsch). It is hidden without JavaScript, when every passage is shown. A choice hides passages designated in other languages; `mixed` passages are always shown. An entry left with nothing to show says which language it is written in and offers to show it.

Inside any Markdown passage, mark the language of a phrase with `[Stimme]{lang=de}`, which becomes `<span lang="de">Stimme</span>`. The phrase must be plain text (no emphasis or links inside) and the tag a language tag such as `de` or `fr`. Phrase languages never add a choice to the control. Inside code, the syntax is left as written.

To add a selectable language, add it to `DisplayLanguage` in `site-builder/src/model.rs` with its code, its own name, and its English name; nothing else needs to change.

Notes and diary entries are documented in `notes/README.md` and `diary/README.md`.
