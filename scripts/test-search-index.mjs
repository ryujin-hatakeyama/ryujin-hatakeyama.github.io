#!/usr/bin/env node
// Tests scripts/search-index.mjs on small made-up sites, never on the
// published content: what it reads, what it refuses, and that the same pages
// always make the same index. Words that must never be searchable are put
// on pages outside the list, and outside the marked text of listed pages.

import assert from "node:assert/strict";
import { mkdir, readdir, readFile, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { buildSearchIndex, decode } from "./search-index.mjs";

const base = path.resolve("target/search-index-test");

const page = (body, chrome = "") =>
  `<!DOCTYPE html><html lang="en"><head><meta charset="UTF-8"><title>Fixture</title></head><body><a class="skip-link" href="#main-content">Skip to content</a><main id="main-content"><nav class="miscellany-sections" aria-label="Miscellany sections"><a href="/miscellany/reading/">Reading</a></nav>${chrome}${body}</main><footer>Last updated</footer></body></html>`;

const SITE = {
  "miscellany/notes/fixture-note/index.html": page(
    `<article class="miscellany-note" data-pagefind-body data-pagefind-filter="section:Notes"><header><h1 data-pagefind-meta="title">合成ノート</h1></header><div lang="ja"><p>公開語彙アルファ。形式意味論の<span lang="de">Urteilskraft</span>を読む。</p></div><p class="language-notice" data-pagefind-ignore="all">Written in Japanese.</p></article>`,
    `<form role="search" aria-label="Search Notes"><input placeholder="Search Notes"></form>`,
  ),
  "miscellany/sketches/fixture-sketch/index.html": page(
    `<article class="miscellany-note" data-pagefind-body data-pagefind-filter="section:Sketches"><header><h1 data-pagefind-meta="title">A made-up sketch</h1><p><time datetime="2026-09-19" data-pagefind-meta="date[datetime]">19 September 2026</time></p></header><p>Public word bravo, and jugement.</p></article>`,
  ),
  "miscellany/diary/2026/10/index.html": page(
    `<div class="diary" data-pagefind-body data-pagefind-filter="section:Diary" data-pagefind-meta="title:October 2026"><article class="diary-entry"><h2 class="diary-heading" id="diary-2026-10-03"><a class="diary-date" href="#diary-2026-10-03" data-pagefind-ignore="all"><time>3 Sat</time></a></h2><div><p lang="ja">雨。公開語彙チャーリー。</p></div></article><article class="diary-entry has-title"><h2 class="diary-heading" id="diary-2026-10-05-evening"><a class="diary-date" href="#diary-2026-10-05-evening" data-pagefind-ignore="all"><time>5 Mon</time></a><span class="diary-title">Evening</span></h2><div><p>Public word delta.</p></div></article></div>`,
    `<nav aria-label="Months"><a href="/miscellany/diary/2026/09/">September 2026</a></nav>`,
  ),
  "miscellany/reading/index.html": page(
    `<ol class="reading-list" data-pagefind-body data-pagefind-filter="section:Reading" data-pagefind-meta="title:Reading"><li><h2 class="reading-title" id="reading-0a1b2c3d"><cite>架空の本</cite></h2><p>Public word echo.</p></li></ol>`,
  ),
  // Never searchable: lists, the Diary's front page, the search page, the
  // encrypted archive, and the rest of the site.
  "miscellany/index.html": page(`<p>listing-only-overview</p>`),
  "miscellany/notes/index.html": page(`<p>listing-only-notes 合成ノート</p>`),
  "miscellany/diary/index.html": page(`<p>listing-only-diary</p>`),
  "miscellany/search/index.html": page(`<p>search-page-only</p>`),
  "archive/index.html": page(`<p>sealed-archive-only 秘匿語彙</p>`),
  "updates/index.html": page(`<p>updates-only</p>`),
};
const NEVER = ["listing-only", "search-page-only", "sealed-archive-only", "秘匿語彙", "updates-only", "Skip to content", "Miscellany sections", "Last updated", "Search Notes", "Written in Japanese", "3 Sat"];
const ALWAYS = ["公開語彙アルファ", "Urteilskraft", "bravo", "jugement", "チャーリー", "delta", "echo"];

async function makeSite(name, files) {
  const root = path.join(base, name);
  await rm(root, { recursive: true, force: true });
  for (const [relative, html] of Object.entries(files)) {
    await mkdir(path.dirname(path.join(root, relative)), { recursive: true });
    await writeFile(path.join(root, relative), html);
  }
  return root;
}

async function indexFiles(root) {
  const out = {};
  const walk = async (directory) => {
    for (const entry of await readdir(directory, { withFileTypes: true })) {
      const full = path.join(directory, entry.name);
      if (entry.isDirectory()) await walk(full);
      else out[path.relative(root, full)] = await readFile(full);
    }
  };
  await walk(path.join(root, "pagefind"));
  return out;
}

const readable = (files) =>
  Object.entries(files)
    .filter(([name]) => /\.pf_(meta|index|fragment|filter)$/.test(name))
    .map(([, bytes]) => decode(bytes).toString("utf8").replace(/​/g, ""))
    .join("\n");

// What is read, and what is not.
const first = await makeSite("site-a", SITE);
const result = await buildSearchIndex(first);
assert.equal(result.pages, 4);
const files = await indexFiles(first);
const text = readable(files);
for (const word of ALWAYS) assert.ok(text.includes(word), `the index should contain ${word}`);
for (const word of NEVER) assert.ok(!text.includes(word), `the index must not contain ${word}`);
assert.ok(text.includes("diary-2026-10-05-evening"), "diary entries keep their anchors");
assert.ok(text.includes("2026-09-19"), "a sketch's date is in its record");
for (const name of Object.keys(files)) {
  assert.ok(!/ui|highlight|playground/.test(name), `${name} should not be published`);
}
const fragments = Object.entries(files).filter(([name]) => name.endsWith(".pf_fragment"));
assert.equal(fragments.length, 4, "one record per page");
const urls = fragments.map(([, bytes]) => JSON.parse(decode(bytes).toString("utf8")).url).sort();
assert.deepEqual(urls, ["/miscellany/diary/2026/10/", "/miscellany/notes/fixture-note/", "/miscellany/reading/", "/miscellany/sketches/fixture-sketch/"]);

// The same pages make the same index, byte for byte.
const second = await makeSite("site-b", SITE);
await buildSearchIndex(second);
assert.deepEqual(await indexFiles(second), files);

// Text marked for the index outside the list stops the build.
const stray = await makeSite("site-stray", { ...SITE, "archive/index.html": page(`<div data-pagefind-body>sealed</div>`) });
await assert.rejects(buildSearchIndex(stray), /archive\/index\.html marks text for the search index but is not a searchable page/);
const listing = await makeSite("site-listing", { ...SITE, "miscellany/notes/index.html": page(`<ol data-pagefind-body>x</ol>`) });
await assert.rejects(buildSearchIndex(listing), /not a searchable page/);
const twice = await makeSite("site-twice", {
  ...SITE,
  "miscellany/notes/fixture-note/index.html": page(`<div data-pagefind-body data-pagefind-filter="section:Notes">a</div><div data-pagefind-body>b</div>`),
});
await assert.rejects(buildSearchIndex(twice), /marks 2 elements/);

// A page filed under another section's filter is refused.
const misfiled = await makeSite("site-misfiled", {
  "miscellany/notes/fixture-note/index.html": page(`<article data-pagefind-body data-pagefind-filter="section:Diary"><h1 data-pagefind-meta="title">x</h1><p>y</p></article>`),
});
await assert.rejects(buildSearchIndex(misfiled), /filed under \["Diary"\]; expected Notes/);

// Nothing published: no index at all.
const empty = await makeSite("site-empty", { "miscellany/index.html": page("<p>x</p>"), "miscellany/reading/index.html": page("<p>No entries yet.</p>") });
assert.equal((await buildSearchIndex(empty)).pages, 0);
await assert.rejects(readdir(path.join(empty, "pagefind")));

// kept for inspection on failure
await rm(base, { recursive: true, force: true });
console.log("Search index tests passed.");
