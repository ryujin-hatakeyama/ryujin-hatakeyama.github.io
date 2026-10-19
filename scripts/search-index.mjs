#!/usr/bin/env node
// Builds the Miscellany search index for a built site and checks it.
//
//   node scripts/search-index.mjs <built-site-directory>
//
// The index is public: every word in it can be read by anyone. So it is made
// from an explicit list of pages, never from the whole site:
//
//  1. Only the pages matching SEARCHABLE_PAGES are read (the same patterns as
//     site-builder/src/validate.rs): the Reading page, each published note
//     and sketch, and each month of the Diary. Drafts are never rendered, so
//     they have no page to read. The encrypted archive, Updates, Research,
//     and every other page are outside the list.
//  2. Each of those pages must mark exactly one element with
//     data-pagefind-body, and no other page may mark one. Only that element
//     is read; navigation and controls lie outside it.
//  3. Before anything is written, every file of the index is decompressed and
//     checked: each page record belongs to a listed page, in the section its
//     address says, without the site's navigation or a local path; and only
//     the files the search page uses are written to <site>/pagefind/.
//
// Japanese is the index language for every page (forceLanguage), so that
// Japanese text is segmented into words. English, German, and French words
// are still found as written, by prefix; there is no stemming.

import { createHash } from "node:crypto";
import { gunzipSync, gzipSync } from "node:zlib";
import { lstat, mkdir, readdir, readFile, rm, writeFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

/** Pages the index may read: keep in step with SEARCHABLE_PAGES in validate.rs. */
export const SEARCHABLE_PAGES = [
  /^miscellany\/reading\/index\.html$/,
  /^miscellany\/notes\/[a-z0-9]+(?:-[a-z0-9]+)*\/index\.html$/,
  /^miscellany\/sketches\/[a-z0-9]+(?:-[a-z0-9]+)*\/index\.html$/,
  /^miscellany\/diary\/[0-9]{4}\/(?:0[1-9]|1[0-2])\/index\.html$/,
];

/** The section filter each address must carry. */
const SECTION_OF = [
  [/^\/miscellany\/reading\/$/, "Reading"],
  [/^\/miscellany\/notes\/[a-z0-9-]+\/$/, "Notes"],
  [/^\/miscellany\/sketches\/[a-z0-9-]+\/$/, "Sketches"],
  [/^\/miscellany\/diary\/[0-9]{4}\/[0-9]{2}\/$/, "Diary"],
];

/** The files of Pagefind's bundle that the search page uses; its own UI is not published. */
const PUBLISHED_FILES = [
  /^pagefind\.js$/,
  /^pagefind-worker\.js$/,
  /^pagefind-entry\.json$/,
  /^wasm\.[a-z]+\.pagefind$/,
  /^pagefind\.ja_[0-9a-f]+\.pf_meta$/,
  /^index\/ja_[0-9a-f]+\.pf_index$/,
  /^fragment\/ja_[0-9a-f]+\.pf_fragment$/,
  /^filter\/ja_[0-9a-f]+\.pf_filter$/,
];

/** Text of the site's own page furniture: it must never be in a record. */
const FURNITURE = ["Skip to content", "Miscellany sections", "Ryujin Hatakeyama home", "Last updated", "Dark mode", "Search Miscellany"];

/** Never in any file of the index. */
const FORBIDDEN = ["/Users/", "/home/runner", "hatakeyama.ryujin.q7@"];

const MARKER = "data-pagefind-body";
const ZERO_WIDTH = /​/g;

async function htmlFiles(root, directory = root) {
  const found = [];
  for (const entry of await readdir(directory, { withFileTypes: true })) {
    const full = path.join(directory, entry.name);
    if (entry.isSymbolicLink()) throw new Error(`symbolic link in the built site: ${path.relative(root, full)}`);
    if (entry.isDirectory()) found.push(...(await htmlFiles(root, full)));
    else if (entry.isFile() && entry.name.endsWith(".html")) found.push(path.relative(root, full).split(path.sep).join("/"));
  }
  return found.sort();
}

/** The listed pages, each checked to mark one element; and no other page marks one. */
export async function searchablePages(site) {
  const pages = [];
  for (const relative of await htmlFiles(site)) {
    const html = await readFile(path.join(site, relative), "utf8");
    const markers = html.split(MARKER).length - 1;
    const listed = SEARCHABLE_PAGES.some((pattern) => pattern.test(relative));
    if (!listed) {
      if (markers > 0) throw new Error(`${relative} marks text for the search index but is not a searchable page`);
      continue;
    }
    if (markers === 0) continue; // an empty Reading list
    if (markers !== 1) throw new Error(`${relative} marks ${markers} elements for the search index; one is allowed`);
    pages.push({ relative, url: `/${relative.replace(/index\.html$/, "")}`, html });
  }
  return pages;
}

/** Decompresses one Pagefind data file: gzip, then the "pagefind_dcd" signature. */
export function decode(bytes) {
  const text = gunzipSync(bytes);
  const signature = "pagefind_dcd";
  if (text.subarray(0, signature.length).toString("latin1") !== signature) throw new Error("unexpected Pagefind data file");
  return text.subarray(signature.length);
}

const SIGNATURE = Buffer.from("pagefind_dcd", "latin1");
const encode = (payload) => gzipSync(Buffer.concat([SIGNATURE, payload]), { level: 9 });

/** The byte length of the CBOR data item at `offset` (the subset Pagefind writes). */
function cborItemLength(bytes, offset) {
  const initial = bytes[offset];
  const major = initial >> 5;
  const info = initial & 0x1f;
  let header = 1;
  let value = info;
  if (info === 24) [header, value] = [2, bytes[offset + 1]];
  else if (info === 25) [header, value] = [3, bytes.readUInt16BE(offset + 1)];
  else if (info === 26) [header, value] = [5, bytes.readUInt32BE(offset + 1)];
  else if (info > 26) throw new Error("unexpected CBOR in the search index");
  if (major === 0 || major === 1) return header;
  if (major === 2 || major === 3) return header + value;
  if (major === 4 || major === 5) {
    let length = header;
    for (let i = 0; i < value * (major === 5 ? 2 : 1); i++) length += cborItemLength(bytes, offset + length);
    return length;
  }
  throw new Error("unexpected CBOR in the search index");
}

/** The items of the CBOR array at `offset`, as byte ranges, and its header. */
function cborArray(bytes, offset) {
  const initial = bytes[offset];
  if (initial >> 5 !== 4 || (initial & 0x1f) > 23) throw new Error("unexpected CBOR in the search index");
  const items = [];
  let at = offset + 1;
  for (let i = 0; i < (initial & 0x1f); i++) {
    const length = cborItemLength(bytes, at);
    items.push(bytes.subarray(at, at + length));
    at += length;
  }
  return { header: bytes.subarray(offset, offset + 1), items, end: at };
}

const shortHash = (bytes, length) => createHash("sha256").update(bytes).digest("hex").slice(0, length);

/**
 * Pagefind 1.5.2 writes the values of a filter in a different order on each
 * run (a Rust HashMap), and names its files by their content, so two builds
 * of the same pages differ. The order carries no meaning: each value lists
 * its own pages. So each filter file's values are put in byte order, and the
 * filter and metadata files are renamed by their new content, with the
 * references to them changed to match. Nothing else is touched.
 */
export function canonicalize(files) {
  const named = files.map((file) => ({ name: file.path.split(path.sep).join("/").replace(/^\/+/, ""), content: Buffer.from(file.content) }));
  const renames = new Map();
  for (const file of named) {
    if (!file.name.endsWith(".pf_filter")) continue;
    const payload = decode(file.content);
    // [key, [[value, [page, …]], …]]
    const outer = cborArray(payload, 0);
    if (outer.items.length !== 2 || outer.end !== payload.length) throw new Error(`unexpected filter file ${file.name}`);
    const key = outer.items[0];
    const values = cborArray(outer.items[1], 0);
    const sorted = [...values.items].sort(Buffer.compare);
    const canonical = Buffer.concat([outer.header, key, values.header, ...sorted]);
    const oldId = file.name.replace(/^filter\//, "").replace(/\.pf_filter$/, "");
    const newId = `${oldId.split("_")[0]}_${shortHash(canonical, oldId.split("_")[1].length)}`;
    renames.set(oldId, newId);
    file.name = `filter/${newId}.pf_filter`;
    file.content = encode(canonical);
  }
  for (const file of named) {
    if (!file.name.endsWith(".pf_meta")) continue;
    let payload = decode(file.content);
    for (const [oldId, newId] of renames) {
      const at = payload.indexOf(Buffer.from(oldId, "latin1"));
      if (at < 0 || newId.length !== oldId.length) throw new Error(`the search metadata does not name ${oldId}`);
      payload = Buffer.concat([payload.subarray(0, at), Buffer.from(newId, "latin1"), payload.subarray(at + oldId.length)]);
    }
    const oldId = file.name.replace(/^pagefind\./, "").replace(/\.pf_meta$/, "");
    const newId = `${oldId.split("_")[0]}_${shortHash(payload, oldId.split("_")[1].length)}`;
    file.name = `pagefind.${newId}.pf_meta`;
    file.content = encode(payload);
    const entry = named.find((other) => other.name === "pagefind-entry.json");
    if (!entry) throw new Error("the search index has no pagefind-entry.json");
    const text = entry.content.toString("utf8");
    if (text.split(`"${oldId}"`).length !== 2) throw new Error("the search index entry does not name its metadata once");
    entry.content = Buffer.from(text.replace(`"${oldId}"`, `"${newId}"`), "utf8");
  }
  return named.map((file) => ({ path: file.name, content: file.content }));
}

/** Checks every file of an index against the pages it was made from. */
export function checkIndex(files, pages) {
  const expected = new Set(pages.map((page) => page.url));
  const seen = new Set();
  const published = [];
  for (const file of files) {
    const name = file.path.split(path.sep).join("/").replace(/^\/+/, "");
    if (!PUBLISHED_FILES.some((pattern) => pattern.test(name))) continue;
    const bytes = Buffer.from(file.content);
    const readable = /\.pf_(meta|index|fragment|filter)$/.test(name) ? decode(bytes) : bytes;
    const text = readable.toString("utf8");
    for (const forbidden of FORBIDDEN) {
      if (name.startsWith("pagefind") && name.endsWith(".js")) break; // Pagefind's own program
      if (text.includes(forbidden)) throw new Error(`the search index file ${name} contains “${forbidden}”`);
    }
    if (name === "pagefind-entry.json") {
      const languages = Object.keys(JSON.parse(text).languages ?? {});
      if (languages.join(",") !== "ja") throw new Error(`the search index has languages ${languages.join(", ")}; expected one index`);
    }
    if (name.endsWith(".pf_fragment")) {
      const record = JSON.parse(text);
      if (!expected.has(record.url)) throw new Error(`the search index has a record for ${record.url}, which is not a searchable page`);
      if (seen.has(record.url)) throw new Error(`the search index has two records for ${record.url}`);
      seen.add(record.url);
      const section = SECTION_OF.find(([pattern]) => pattern.test(record.url))?.[1];
      const filters = record.filters?.section ?? [];
      if (!section || filters.length !== 1 || filters[0] !== section) {
        throw new Error(`${record.url} is filed under ${JSON.stringify(filters)}; expected ${section}`);
      }
      if (!record.meta?.title?.trim()) throw new Error(`${record.url} has no title in the search index`);
      const content = String(record.content).replace(ZERO_WIDTH, "");
      for (const furniture of FURNITURE) {
        if (content.includes(furniture)) throw new Error(`${record.url}: the search index read the page's “${furniture}”`);
      }
    }
    published.push({ name, bytes });
  }
  for (const url of expected) if (!seen.has(url)) throw new Error(`${url} is missing from the search index`);
  if (!published.some((file) => file.name === "pagefind.js")) throw new Error("the search index has no pagefind.js");
  return published;
}

/** Builds, checks, and writes the index of `site`; returns what was indexed. */
export async function buildSearchIndex(site) {
  const pages = await searchablePages(site);
  const output = path.join(site, "pagefind");
  const existing = await lstat(output).catch(() => null);
  if (existing?.isSymbolicLink()) throw new Error("the search index directory is a symbolic link");
  await rm(output, { recursive: true, force: true });
  // Nothing published yet: no index at all, and the search page says that
  // nothing was found.
  if (pages.length === 0) return { pages: 0, files: 0, bytes: 0 };
  const pagefind = await import("pagefind");
  try {
    // KaTeX writes each formula twice, as MathML (with its TeX) for
    // assistive technology and as visible HTML; only the visible form is read.
    const { errors, index } = await pagefind.createIndex({ forceLanguage: "ja", keepIndexUrl: false, writePlayground: false, excludeSelectors: [".katex-mathml"] });
    if (errors.length || !index) throw new Error(`Pagefind: ${errors.join("; ")}`);
    for (const page of pages) {
      const added = await index.addHTMLFile({ sourcePath: page.relative, content: page.html });
      if (added.errors.length) throw new Error(`Pagefind could not read ${page.relative}: ${added.errors.join("; ")}`);
      if (added.file.url !== page.url) throw new Error(`Pagefind filed ${page.relative} at ${added.file.url}`);
    }
    const { errors: fileErrors, files } = await index.getFiles();
    if (fileErrors.length) throw new Error(`Pagefind: ${fileErrors.join("; ")}`);
    const published = checkIndex(canonicalize(files), pages);
    for (const file of published) {
      const target = path.join(output, file.name);
      await mkdir(path.dirname(target), { recursive: true });
      await writeFile(target, file.bytes);
    }
    return { pages: pages.length, files: published.length, bytes: published.reduce((sum, file) => sum + file.bytes.length, 0) };
  } finally {
    await pagefind.close();
  }
}

if (process.argv[1] && path.resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const site = process.argv[2];
  if (!site || process.argv.length > 3) {
    console.error("usage: node scripts/search-index.mjs <built-site-directory>");
    process.exit(2);
  }
  try {
    const result = await buildSearchIndex(path.resolve(site));
    console.log(`Indexed ${result.pages} Miscellany page(s) for search: ${result.files} files, ${(result.bytes / 1024).toFixed(1)} KiB.`);
  } catch (error) {
    console.error(`search index failed: ${error.message}`);
    process.exit(1);
  }
}
