# Ryujin Hatakeyama — personal website

A static, English-language personal and academic website built with Astro and a small OCaml content validator. It contains no invented publications, updates, affiliations, contact details, or literary works.

## Prerequisites

- Node.js 24 (the current Astro toolchain requires Node 22.12 or newer)
- npm
- opam 2.1 or newer
- OCaml 5.2 or newer
- Dune 3.16 or newer

Use a project-local opam switch so this site never changes a research or MetaOCaml switch:

```sh
opam switch create . --packages=ocaml-system
eval "$(opam env)"
opam install . --deps-only --with-test
npm ci
```

The local switch lives in `_opam/` and is ignored by Git.

## Run and build

```sh
npm run dev       # validate updates, then start Astro
npm run build     # validate updates, then create dist/
npm run preview   # serve the production build
npm run verify    # OCaml tests, Astro diagnostics, production build
```

The production origin is configured in `astro.config.mjs`:

`https://ryujin-hatakeyama.github.io/`

This is a GitHub Pages user site, so there is intentionally no repository-name `base` prefix.

## Add an update

Create one JSON file in `content/updates/`. The complete field example and award model are in [content/updates/README.md](content/updates/README.md). Then run:

```sh
npm run generate
```

The OCaml component parses and validates all source files, rejects duplicate IDs, invalid calendar dates, unknown or duplicate categories, malformed links, missing English text, invalid slugs, unknown event kinds, and ambiguous award outcomes. `draft` records are validated but not exported; only `published` records reach Astro. Astro validates the generated JSON again before rendering it.

Static types cannot prove facts in external files. The parser can guarantee that a date is structurally and calendrically valid, but not that an event actually happened; editorial verification remains essential. Draft source files are still visible in a public Git repository, so confidential material must never be committed.

## Add a publication or presentation

Create a JSON file in `src/data/publications/`:

```json
{
  "slug": "stable-record-slug",
  "title": "Verified title",
  "authors": ["Author names in publication order"],
  "year": 2026,
  "type": "conference-paper",
  "status": "published",
  "venue": "Verified venue",
  "links": [
    { "label": "DOI", "url": "https://doi.org/...", "type": "doi" }
  ],
  "projectIds": [],
  "draft": true
}
```

Supported record types distinguish conference papers, journal articles, preprints, extended abstracts, workshop contributions, student research competition entries, presentations, posters, and software. Status is independently modeled as `draft`, `submitted`, `accepted`, `forthcoming`, `published`, or `presented`. Presentation and poster records also require structured `presentation` metadata for the format, optional category, and any verified proceedings-review status. Keep `draft: true` until every claim and link is verified.

## Add a writing

Create a Markdown or MDX file in `src/content/writings/`:

```md
---
slug: a-stable-slug
title: "The authored title"
description: "A concise description"
date: 2026-01-01
lang: en
kind: essay
math: false
draft: true
---

The text begins here.
```

Allowed kinds are `essay`, `note`, `literary`, `prose`, and `fragment`. Markdown supports footnotes, block quotations, Shiki-highlighted code, Unicode mathematics, and TeX math. KaTeX styles are included only in the long-form layout, not on the homepage or index pages.

Do not translate personal or literary writing unless the author explicitly authorizes the translation.

## Translation infrastructure (currently inactive)

The public site is currently English-only: it emits no `/ja/` routes, language switch, or `hreflang` metadata. The former Japanese page sources are preserved under `src/inactive-pages/ja/` for possible future reuse. The content schemas still accept optional Japanese fields and `translationKey` values; restoring public translations requires deliberately moving and reviewing those page sources.

Updates require English text and may add Japanese `title`, `summary`, and `body` values. When Japanese text is absent, the Japanese archive shows the English original.

## Author information and name pronunciation

Edit `src/data/site.ts` only after the author confirms the information. The Japanese name, reading, and provisional segmental IPA are displayed directly beneath the romanized name. There is intentionally no pronoun field or generated pronunciation audio.

Affiliation, contact, and academic-profile arrays are empty until verified. Layout components must not be edited when those values are supplied.

## Verification checklist

1. Run `npm run verify` (this also checks generated routes and internal links).
2. Inspect `/`, `/cv/`, `/research/`, `/writings/`, and `/updates/`; confirm no `/ja/` pages are generated.
3. Check System, Light, and Dark settings with keyboard navigation.
4. Check `/updates/research/`, `/updates/academia/`, and `/updates/writing/` without JavaScript.
5. Confirm draft records do not appear in `src/data/generated/updates.json` or `dist/`.
6. Inspect mobile and desktop widths, long titles, focus states, and overflow.
7. Confirm canonical metadata, sitemap, RSS, and `robots.txt` use `https://ryujin-hatakeyama.github.io/` without a repository-name base prefix.
8. Verify every public claim, URL, PDF permission, and authored text before deployment.

The visual studies used during design review remain under `src/inactive-pages/` and are not included in the public build.

## Deployment

`.github/workflows/deploy.yml` follows Astro’s current GitHub Pages action pattern and also installs OCaml so generated update data can never depend on an undocumented local artifact. It uses Node.js 24, installs the opam dependencies, and runs `npm run verify` through the opam environment before uploading `dist/`. Deployment is manual (`workflow_dispatch`) to prevent accidental publication; no repository variables or secrets are required.

Before the first deployment, set **Settings → Pages → Build and deployment → Source** to **GitHub Actions**. After the reviewed source has been pushed to `main`, open **Actions → Deploy to GitHub Pages → Run workflow**. Running that workflow is the publication action.

`dist/` is ordinary static output and can instead be copied to Sakura Internet or any other static host. No runtime server, database, CMS, analytics, or third-party script is required.
