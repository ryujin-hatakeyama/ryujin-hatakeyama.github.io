# Miscellany

Source for `/miscellany/` and its four sections. A published note, diary entry, sketch, or reading item that states the day it was published is announced in Updates (see below); nothing else here is. Nothing here is in the RSS feed, which lists update records only.

| Section | Source | Route |
|---|---|---|
| Reading | `reading.yaml` | `/miscellany/reading/` |
| Notes | `notes/*.md` | `/miscellany/notes/` and `/miscellany/notes/<slug>/` |
| Diary | `diary/*.md` | `/miscellany/diary/` (months) and `/miscellany/diary/<year>/<month>/` |
| Sketches | `sketches/*.md` | `/miscellany/sketches/` and `/miscellany/sketches/<slug>/` |

Published Reading, Notes, Diary, and Sketches are searchable at `/miscellany/search/` (see `docs/search.md`). Only those pages are indexed: drafts are never rendered, and no other page of the site is read by the index.

## Visited links

Linked titles in Reading and article-title links in the Sketches list use the browser's standard `:visited` state. A visited title uses the site's existing `--ink-soft` grey; its underline, typography, hover, and focus treatment stay unchanged. Reading entries without a `url` remain plain text.

The selectors are deliberately limited to `.reading-list .reading-title a` and `.sketch-list > li > h2 > a`. Notes, Diary, search results, Updates, navigation, breadcrumbs, and links inside articles do not receive visited styling. There is no JavaScript or site-managed storage for reading or visit state.

## Announcing in Updates

Updates lists the author's update records (`content/updates/`) and the published Miscellany articles as one timeline, on the homepage, on `/updates/` and its Writing page, and beside the sections on `/miscellany/` on a wide screen. The RSS feed is not part of it: it lists update records only. An article is listed when its source states `published: YYYY-MM-DD`, the day it was published on the site:

```yaml
published: 2026-10-14
```

Its entry reads `Published “Title” in Notes.`, with the title exactly as written and linked to the article (an untitled diary entry: `Published an entry for 2 October 2026 in Diary.`). It is in the Writing category and is listed by that day among the update records. Nothing is stored for it; it is derived from the article at every build, so:

- An article without `published` is not announced. No other date stands in for it: not a diary entry's or sketch's `date` (the day it records or was written), not a Git commit, deployment, build, file, or edit time, and not the time it was published from the Writing Studio.
- Correcting `published` moves the entry; changing the title renames it. Rebuilding or publishing the same article again never adds a second entry.
- Drafts are never announced, nor is anything not in this directory.
- An article whose `published` day is still to come (in Asia/Tokyo) is not announced until a build on or after that day. The article itself is published as usual: `published` only places the announcement.
- The entry's id, and its anchor on `/updates/`, come from the article's identity: `published-notes-<slug>`, `published-sketches-<slug>`, `published-diary-<date>[-<slug>]`, and `published-reading-<id>`. Update records may not use these ids.

`published` belongs to the article, in the main file; companion passages cannot carry it. Reading items need a stable `id` to be announced (see below).

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
    id: stable-slug                # optional: a stable identity; required with published
    published: 2026-10-14          # optional: the day it was published, to announce it in Updates
```

Only `title` is required. An item with an `id` (lowercase letters, digits, and single hyphens, unique in the list) is anchored at `#reading-<id>`, which survives a corrected title; other items are anchored by a hash of their title and author. `author` may be omitted for anonymous works. `lang` describes the work: its title and author's name, in any language tag such as `ja`, `de`, or `zh-Hant`; omit it for English. Keep original titles and names; do not translate them. Unknown fields are rejected.

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

Notes, diary entries, sketches, and reading commentary may be written in several languages. Each separately authored text is a *passage* with its own language; an article with several passages is still one article, with one URL or anchor and one entry in every list. The texts need not be translations of one another and need not share structure.

When published passages use more than one language, Miscellany pages offer a small control: All, then each language used (English, 日本語, Deutsch). It is hidden without JavaScript, when every passage is shown. A choice hides passages designated in other languages; `mixed` passages are always shown. An entry left with nothing to show says which language it is written in and offers to show it.

Inside any Markdown passage, mark the language of a phrase with `[Stimme]{lang=de}`, which becomes `<span lang="de">Stimme</span>`. The phrase must be plain text (no emphasis or links inside) and the tag a language tag such as `de` or `fr`. Phrase languages never add a choice to the control. Inside code, the syntax is left as written.

To add a selectable language, add it to `DisplayLanguage` in `site-builder/src/model.rs` with its code, its own name, and its English name; nothing else needs to change.

## Code

A fenced code block that names its language is highlighted when the site is built: `rocq` (or `coq`), `haskell` (`hs`), `ocaml` (`ml`), `metaocaml` (`ber-metaocaml`, read as OCaml), `lean` (`lean4`), `rust`, `typescript`, `python`, `bash`, `latex`, `bibtex`, `json`, `yaml`, and the other languages of Shiki's grammar collection. A fence with no language, or `text`, stays plain. Any other name stops the build, so a typo is never left plain unnoticed. Inline code is never highlighted.

## Music and podcasts

In a note, diary entry, or sketch, a paragraph that holds nothing but one address, or two addresses of the same work on two services (on two lines, or separated by a space), becomes a media card: the work's title, a second line, its cover, and "Open" for the chosen service. Recognised: songs, albums, and playlists at `https://music.apple.com/…` and `https://open.spotify.com/…`, and podcasts and episodes at `https://podcasts.apple.com/…` and `https://open.spotify.com/show/…` or `/episode/…`. A podcast card also has the service's own player; music never plays on the page. An address anywhere else (in a sentence, a list, a quotation, a footnote) stays text.

The details come from the file's front matter, written by the Writing Studio when it publishes, one entry per address:

```yaml
music:
  - {"url": "https://music.apple.com/jp/album/x/1440806071?i=1440806402", "title": "…", "artist": "…", "artwork": "https://is1-ssl.mzstatic.com/…"}
```

Podcasts go under `podcasts:`, with `show` (an episode's show) in place of `artist`:

```yaml
podcasts:
  - {"url": "https://podcasts.apple.com/us/podcast/x/id1200361736?i=1000797757135", "title": "…", "show": "…", "detail": "23 Oct 2026 · 30 min"}
```

`url` is the address without tracking parameters; `artist` or `show`, `detail` (a further line, such as a song’s album), and `artwork` are optional, and `artwork` must be on the services' own image servers. Without an entry, the card says what it is (“A song on Spotify”, “An episode on Apple Podcasts”) and still links there. The build never asks any service anything; see `docs/media.md`.

## Theorem environments

In a note, diary entry, or sketch, theorems, lemmas, definitions, proofs, and their kin are written as Quarto fenced divs and referred to by label:

```markdown
::: {#lem-step name="Measure"}
Every step shrinks the measure.
:::

::: {.proof of="lem-step"}
The measure is a natural number.
:::

By @lem-step, evaluation terminates.
```

The kind comes from the label's prefix: `thm` Theorem, `lem` Lemma, `prp` Proposition, `cor` Corollary, `cnj` Conjecture, `clm` Claim, `obs` Observation, `def` Definition, `asm` Assumption, `exm` Example, `exr` Exercise, `rem` Remark; or from a class (`.lemma`) for one without a label. A proof (`.proof`) takes no label and may say what it proves with `of="…"`. `name="…"` adds a name in parentheses, and `.unnumbered` leaves the number out. A label is the prefix, a hyphen, and letters, digits and `_` joined by single `-`, `:` or `.`; `@label` refers to it (not after a letter or digit, and not as a citation `[@label]`).

All numbered kinds share one counter, numbered within sections (`2.1`, `2.2`, `3.1`; the sections are the text's highest heading level, and anything before the first is section 0) or, with `theoremNumbering: document` in the front matter, through the text (`1`, `2`, `3`). The numbers are never written into the text. A proof ends with ∎, once: not when the text already ends with ∎ or a formula with `\blacksquare`, and a final `\qed` written as text becomes the mark.

The build refuses, by line, a malformed opening line, an environment never closed or opened inside another, a `:::` line that closes nothing, a label given twice, and a reference or `of` to a label no environment has. Fenced code is never read for any of this. The rules, and the fixtures shared with the Writing Studio that hold both to the same meaning, are in `site-builder/src/theorems.rs` and `site-builder/tests/fixtures/theorems/`.

Notes, diary entries, and sketches are documented in `notes/README.md`, `diary/README.md`, and `sketches/README.md`.
