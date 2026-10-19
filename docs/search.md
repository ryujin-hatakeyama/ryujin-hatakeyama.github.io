# Miscellany search

`/miscellany/search/` searches the published Reading, Notes, Diary, and Sketches. Nothing else on the site is searchable: not Research, the CV, Updates, Writings, or the encrypted archive.

## How it is built

The index is static, made after the site is built and deployed with it: no server and no third-party service. It is made by [Pagefind](https://pagefind.app/), pinned exactly in `package.json` (`pagefind` 1.5.2, with its platform binaries in `package-lock.json`), from the Node.js script `scripts/search-index.mjs`:

```sh
node scripts/search-index.mjs dist
```

`./scripts/build-site.sh` runs it after the Rust build and then checks the whole site again (`check-dist`); `npm run verify` also indexes the second, comparison build, compares the two byte for byte, and runs `scripts/test-search-index.mjs`. GitHub Actions runs `npm run verify`, so the deployed index is the one that was checked.

## What is indexed

The index is public: anyone can download it and read every word in it. So it is made from an explicit list, never from the whole site.

1. **Only these built pages are read** (`SEARCHABLE_PAGES`, the same patterns in `scripts/search-index.mjs` and `site-builder/src/validate.rs`): `miscellany/reading/`, `miscellany/notes/<slug>/`, `miscellany/sketches/<slug>/`, and `miscellany/diary/<year>/<month>/`. Drafts are never rendered, so they have no page to read.
2. **Only the marked text of each page is read.** Each of those pages marks one element with `data-pagefind-body` (the entries, the article, the reading list) and its section with `data-pagefind-filter="section:…"`. Navigation, the search fields, the language control, and its notices are outside the element or excluded. No other page may carry the mark: the build fails if one does (`validate_search_markers`, and again in the indexer).
3. **Every file is checked before it is written.** The indexer decompresses each file of the index: each page record must belong to a listed page, filed under the section its address says, with a title, without the site's navigation, local paths, or the protected email address. Only the files the search page uses are published (`pagefind/pagefind.js`, its worker, entry, WebAssembly, metadata, and the `index/`, `fragment/`, and `filter/` files); Pagefind's own interface is not.

Each text is indexed once: a note or sketch on its own page, a diary entry on its month's page (the Diary's front page lists months only), a reading item on the Reading page. Overview and section indexes are not indexed.

When nothing has been published, there is no index; the search page then says that nothing was found.

## Languages

Every page is indexed as Japanese (`forceLanguage: "ja"`), so Japanese text is divided into words; a query is divided the same way. English, German, and French words are found as written, from their beginning (`stag` finds `staged`); there is no stemming. Formulas are indexed as they are shown, not as TeX (KaTeX's MathML copy is excluded). Markdown is never read: the index is made from the built pages.

## Results

Results are in Pagefind's order of relevance. A Diary month is split into its entries and the Reading page into its works, using the anchors of the page (`diary-YYYY-MM-DD[-slug]`, `reading-<hash>`), so each result leads to the entry or work itself, and each entry appears once. Each result shows its title (a diary entry's date, and its title if it has one), its section, a sketch's date, and a short excerpt with the matched words marked.

The query and section are kept in the address (`?q=…&section=diary`), so a reload or a shared link repeats the search. A section's own search field (on its index and, for the Diary, on each month) starts with that section chosen. Without JavaScript the forms still lead to the search page, which says that searching needs JavaScript.

## Reproducibility

Pagefind 1.5.2 writes the values of a filter in a different order on each run, and names files by their content, so two builds of the same pages would differ. The indexer puts the filter values in byte order and renames the affected files by their new content (`canonicalize` in the script). Nothing else is changed; the result is identical on every run, and `npm run verify` compares it.

## Changing the Pagefind version

Change the exact version in `package.json`, run `npm install`, and run `npm run verify`. Check that `scripts/test-search-index.mjs` still passes (it covers the file layout, the data format the checks decode, and reproducibility), and look at the search page in a browser.
