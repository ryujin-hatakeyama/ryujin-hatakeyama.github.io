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

- typed publication, presentation, update-export, writing, and project models;
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
- decoding the numeric email attribute only after user activation.

CSS remains CSS. The prior stylesheet is copied rather than embedded in Rust. Font and KaTeX files are ordinary public assets.

## Determinism and failure behavior

The Rust build cleans only an explicitly allowed output (`dist` or a path below `target`), reads content in sorted path order, applies stable secondary sort keys, uses a fixed copyright year matching the migrated site, and writes fixed route paths. Verification builds twice and compares every output path and byte.

Required source data and assets fail closed. The public audit rejects source maps, unrecognized extensions, `.git` content, cleartext protected email, fixture text, and local absolute paths.
