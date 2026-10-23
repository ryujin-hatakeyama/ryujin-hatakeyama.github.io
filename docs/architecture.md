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
- the Updates timeline (`site-builder/src/timeline.rs`): the exported update records joined by an entry for each published article that states its publication day, derived at every build and shared by the homepage, the Updates pages, and the Miscellany overview (the RSS feed lists update records only);
- cross-record validation and deterministic ordering;
- explicit static route construction;
- reusable Maud layout and page components;
- Markdown, footnote, and KaTeX rendering;
- syntax highlighting of fenced code at build time (`src/highlight.rs`): a fence's language (the site's names, such as `rocq`, `metaocaml`, and `hs`, then the bundled grammars' own) is tokenized by irosashi, a Rust port of vscode-textmate with Shiki's TextMate grammars, pinned exactly, and written as Shiki-form HTML coloured by its bundled VS Code Light+ and Dark+ themes; an unlabelled or `text` fence stays plain, and an unknown name stops the build;
- media cards (`src/media/`): which music and podcast addresses are recognised, how each is rebuilt, the players' addresses, the front matter's details (`music:`, `podcasts:`), and the card's static HTML (`docs/media.md`);
- metadata, RSS, sitemap, robots, 404, and asset generation;
- internal-link, anchor, favicon, accessibility-sensitive markup, and public-artifact checks.

OCaml (Updates only; it knows nothing of Miscellany, media, or the Writing Studio):

- parsing update source JSON, including duplicate-key rejection;
- validating update slugs, real dates, categories, event kinds, award outcomes, links, and visibility;
- excluding drafts and exporting deterministically ordered public JSON;
- its existing unit tests.

TypeScript:

- initial system/light/dark resolution and the appearance button;
- decoding the numeric email attribute only after user activation;
- the Miscellany search page, which reads the static search index (below), and the forwarding of earlier Diary anchors to their month's page;
- media cards' interaction only (`client/media.ts`, loaded by `client/site.ts` only on a page whose card offers two services): the choice of service, where "Open" goes, and which one player a podcast card holds. It parses no address; the markup Rust wrote is its only input.

TypeScript does no syntax highlighting: the browser gets highlighted code as static HTML, each token's light colour inline and its dark one in a CSS variable that the stylesheet uses with the dark theme.

The TypeScript is compiled by `tsc` alone, with no bundler and no framework: `site.js` is a classic script (it must run in `<head>` to set the theme before the page is drawn), and `media.js` and `diary.js` are ES modules it imports only on pages that need them.

Node.js (scripts/search-index.mjs):

- building the Miscellany search index with Pagefind, pinned exactly, from an explicit list of built pages, and checking every file of it before it is written. See `docs/search.md`.

The Writing Studio (a separate, private application) owns authoring and enrichment: it finds the details of media addresses and writes them into the published file's front matter, which the Rust build only checks and renders.

Irosashi (MIT) and the grammars it embeds (each under its own licence) run only in the builder, which needs Rust 1.94 for them (`rust-toolchain.toml`); nothing of them is served.

CSS remains CSS. The prior stylesheet is copied rather than embedded in Rust. Font and KaTeX files are ordinary public assets.

## Determinism and failure behavior

The Rust build cleans only an explicitly allowed output (`dist` or a path below `target`), reads content in sorted path order, applies stable secondary sort keys, uses a fixed copyright year matching the migrated site, and writes fixed route paths. Verification builds twice, indexes both builds for search, and compares every output path and byte.

The one input not taken from the repository is the build day, a calendar day in Asia/Tokyo, which places each update under Upcoming (marked Ongoing while the event runs), Recent Updates, or Previously announced. It is read once per build from `SITE_BUILD_DATE` (`YYYY-MM-DD`) when set, and otherwise from the system clock at UTC+09:00, never from the machine's time zone. It affects only the homepage and the Updates listing pages, never anchors or other routes; the one exception is an article whose publication day is after the build day, which the Updates pages and the Miscellany overview leave out until then; given the same sources and day, the output is identical, and verification fixes the day for both builds it compares. See `docs/update-dates.md`.

Structural errors fail the build: invalid JSON, schema violations, impossible dates, an end before the start, duplicate update ids, and broken links or anchors. An editorial matter, a planned update whose event has ended, is reported as a warning (and a GitHub Actions annotation) without stopping the build, so it never blocks unrelated pages.

Required source data and assets fail closed. The public audit rejects source maps, unrecognized extensions, `.git` content, cleartext protected email, fixture text, and local absolute paths.
