# Rust-first architecture decision

## Decision

Use a small purpose-built static site generator around Maud rather than Maudit.

Maudit was evaluated on 2026-10-09. Its current 0.12.x API provides route macros, Maud integration, asset processing, sitemap support, an incremental cache, and a CLI/dev server. It is active and technically capable, but this site has a small explicit route set and does not need framework routing, incremental dependency tracking, asset prefetching, or an HTTP server. Direct Maud keeps the dependency surface and generated behavior easier to audit, and avoids binding the site to a pre-1.0 framework API.

References checked:

- <https://github.com/bruits/maudit>
- <https://docs.rs/maudit/latest/maudit/>
- <https://docs.rs/maudit/latest/maudit/struct.BuildOptions.html>

## Responsibility boundaries

Rust:

- typed publication, presentation, update-export, writing, project, and Miscellany (reading, note, diary, sketch) models, with the Diary shown by month from one file per entry;
- cross-record validation and deterministic ordering;
- explicit static route construction;
- reusable Maud layout and page components;
- Markdown, footnote, and KaTeX rendering;
- metadata, RSS, sitemap, robots, 404, and asset generation;
- internal-link, anchor, favicon, accessibility-sensitive markup, and public-artifact checks.

OCaml:

- parsing update source JSON, including duplicate-key rejection;
- validating update slugs, real dates, categories, event kinds, award outcomes, links, and visibility;
- excluding drafts and exporting deterministically ordered public JSON;
- its existing unit tests.

TypeScript:

- initial system/light/dark resolution and the appearance button;
- decoding the numeric email attribute only after user activation;
- the Miscellany search page, which reads the static search index (below), and the forwarding of earlier Diary anchors to their month's page.

Node.js (scripts/search-index.mjs):

- building the Miscellany search index with Pagefind, pinned exactly, from an explicit list of built pages, and checking every file of it before it is written. See `docs/search.md`.

CSS remains CSS. The prior stylesheet is copied rather than embedded in Rust. Font and KaTeX files are ordinary public assets.

## Determinism and failure behavior

The Rust build cleans only an explicitly allowed output (`dist` or a path below `target`), reads content in sorted path order, applies stable secondary sort keys, uses a fixed copyright year matching the migrated site, and writes fixed route paths. Verification builds twice, indexes both builds for search, and compares every output path and byte.

The one input not taken from the repository is the build day, a calendar day in Asia/Tokyo, which places each update under Upcoming (marked Ongoing while the event runs), Recent Updates, or Previously announced. It is read once per build from `SITE_BUILD_DATE` (`YYYY-MM-DD`) when set, and otherwise from the system clock at UTC+09:00, never from the machine's time zone. It affects only the homepage and the Updates listing pages, never the RSS feed, anchors, or other routes; given the same sources and day, the output is identical, and verification fixes the day for both builds it compares. See `docs/update-dates.md`.

Structural errors fail the build: invalid JSON, schema violations, impossible dates, an end before the start, duplicate update ids, and broken links or anchors. An editorial matter, a planned update whose event has ended, is reported as a warning (and a GitHub Actions annotation) without stopping the build, so it never blocks unrelated pages.

Required source data and assets fail closed. The public audit rejects source maps, unrecognized extensions, `.git` content, cleartext protected email, fixture text, and local absolute paths.
