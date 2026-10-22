type Appearance = 'light' | 'dark' | null;

const media = window.matchMedia('(prefers-color-scheme: dark)');

function readAppearance(): Appearance {
  try {
    const stored = window.localStorage.getItem('appearance');
    return stored === 'light' || stored === 'dark' ? stored : null;
  } catch {
    return null;
  }
}

function applyAppearance(choice: Appearance, button?: HTMLButtonElement): void {
  const resolved = choice ?? (media.matches ? 'dark' : 'light');
  document.documentElement.dataset.theme = resolved;
  document.documentElement.dataset.appearance = choice ?? 'system';
  document.documentElement.style.colorScheme = resolved;

  // The button keeps the stable name "Dark mode"; only its pressed state changes.
  button?.setAttribute('aria-pressed', String(resolved === 'dark'));
}

// Apply the initial choice while the document is still in <head>, preventing a
// light-theme flash for dark-theme visitors.
applyAppearance(readAppearance());

document.addEventListener('DOMContentLoaded', () => {
  const button = document.querySelector<HTMLButtonElement>('#appearance-toggle');
  applyAppearance(readAppearance(), button ?? undefined);

  button?.addEventListener('click', () => {
    const choice: Exclude<Appearance, null> =
      document.documentElement.dataset.theme === 'dark' ? 'light' : 'dark';
    try {
      window.localStorage.setItem('appearance', choice);
    } catch {
      // The control still works for this page view when storage is unavailable.
    }
    applyAppearance(choice, button);
  });

  media.addEventListener?.('change', () => {
    if (readAppearance() === null) applyAppearance(null, button ?? undefined);
  });

  // The publication language filter is hidden in the generated HTML, so the
  // full list remains available without JavaScript. Filtering only hides
  // entries; the newest-first order of the list is unchanged.
  for (const filter of document.querySelectorAll<HTMLElement>('[data-publication-filter]')) {
    const select = filter.querySelector('select');
    const list = select && document.getElementById(select.getAttribute('aria-controls') ?? '');
    const status = filter.querySelector<HTMLElement>('[data-publication-filter-status]');
    if (!select || !list) continue;
    const entries = Array.from(list.querySelectorAll<HTMLElement>(':scope > li[data-language]'));
    const apply = (announce: boolean): void => {
      let shown = 0;
      for (const entry of entries) {
        entry.hidden = select.value !== 'all' && entry.dataset.language !== select.value;
        if (!entry.hidden) shown += 1;
      }
      if (status && announce) {
        status.textContent = `Showing ${shown} of ${entries.length} publications.`;
      }
    };
    select.addEventListener('change', () => apply(true));
    apply(false);
    filter.hidden = false;
  }

  setUpLanguageFilters();
  forwardDiaryAnchor();
  setUpSearch();
  setUpMusicCards();
  setUpReadingState();
  setUpSiteMessage();

  for (const control of document.querySelectorAll<HTMLElement>('[data-email-code]')) {
    control.addEventListener('click', (event) => {
      event.preventDefault();
      const encoded = control.dataset.emailCode;
      if (!encoded) return;
      const codePoints = encoded.split('-').map(Number);
      if (codePoints.some((value) => !Number.isInteger(value))) return;
      window.location.href = `mailto:${String.fromCodePoint(...codePoints)}`;
    });
  }
});

const LANGUAGE_KEY = 'miscellany-language';

function readLanguage(): string {
  try {
    return window.localStorage.getItem(LANGUAGE_KEY) ?? 'all';
  } catch {
    return 'all';
  }
}

// Miscellany entries may hold separately authored passages in several
// languages. The language control is hidden in the generated HTML, so every
// passage is shown without JavaScript. A choice hides designated passages in
// other languages with the hidden attribute, which also removes them from
// keyboard and screen-reader navigation; mixed-language passages are never
// hidden. An entry with nothing left to show displays its notice instead,
// whose button reveals that entry alone. The choice is remembered in this
// browser only, across Miscellany pages.
function setUpLanguageFilters(): void {
  const filters = Array.from(document.querySelectorAll<HTMLElement>('[data-language-filter]'));
  if (filters.length === 0) return;
  const buttons = filters.flatMap((filter) =>
    Array.from(filter.querySelectorAll<HTMLButtonElement>('button[data-language]')),
  );
  const groups = Array.from(document.querySelectorAll<HTMLElement>('[data-passages]'));
  const offered = new Set(buttons.map((button) => button.dataset.language));
  let choice = readLanguage();
  if (!offered.has(choice)) choice = 'all';

  const apply = (): void => {
    for (const button of buttons) {
      button.setAttribute('aria-pressed', String(button.dataset.language === choice));
    }
    for (const group of groups) {
      const revealed = group.dataset.revealed === 'true';
      let shown = 0;
      for (const passage of group.querySelectorAll<HTMLElement>(':scope > [data-passage]')) {
        const language = passage.dataset.passage;
        passage.hidden =
          !revealed && choice !== 'all' && language !== 'mixed' && language !== choice;
        if (!passage.hidden) shown += 1;
      }
      const notice = group.querySelector<HTMLElement>(':scope > [data-language-notice]');
      if (notice) notice.hidden = shown > 0;
    }
  };

  for (const button of buttons) {
    button.addEventListener('click', () => {
      choice = button.dataset.language ?? 'all';
      try {
        window.localStorage.setItem(LANGUAGE_KEY, choice);
      } catch {
        // The choice still applies to this page view when storage is unavailable.
      }
      for (const group of groups) delete group.dataset.revealed;
      apply();
    });
  }

  for (const reveal of document.querySelectorAll<HTMLButtonElement>('[data-language-reveal]')) {
    reveal.addEventListener('click', () => {
      const group = reveal.closest<HTMLElement>('[data-passages]');
      if (!group) return;
      group.dataset.revealed = 'true';
      apply();
      // The notice and its button are now hidden; continue at the text.
      const first = group.querySelector<HTMLElement>(':scope > [data-passage]');
      if (first) {
        first.tabIndex = -1;
        first.focus();
      }
    });
  }

  apply();
  for (const filter of filters) filter.hidden = false;
}

// Before months had pages, every diary entry was on /miscellany/diary/ at
// #diary-YYYY-MM-DD[-slug]. Such a link now opens the Diary's front page;
// when its month is listed there, the entry is on that month's page under
// the same anchor, so the link is forwarded there.
function forwardDiaryAnchor(): void {
  if (window.location.pathname !== '/miscellany/diary/') return;
  const match = /^#diary-([0-9]{4})-([0-9]{2})-[0-9]{2}(?:-[a-z0-9]+(?:-[a-z0-9]+)*)?$/.exec(
    window.location.hash,
  );
  if (!match) return;
  const month = `/miscellany/diary/${match[1]}/${match[2]}/`;
  if (document.querySelector(`[data-diary-archive] a[href="${month}"]`)) {
    window.location.replace(`${month}${window.location.hash}`);
  }
}

// ---------------------------------------------------------------- search

interface PagefindAnchor {
  element: string;
  id: string;
  text: string;
  location: number;
}

interface PagefindSubResult {
  title: string;
  url: string;
  excerpt: string;
  locations: number[];
  anchor?: PagefindAnchor;
}

interface PagefindData {
  url: string;
  excerpt: string;
  meta: Record<string, string | undefined>;
  filters: Record<string, string[] | undefined>;
  sub_results: PagefindSubResult[];
  anchors: PagefindAnchor[];
}

interface PagefindResult {
  data(): Promise<PagefindData>;
}

interface Pagefind {
  options(options: Record<string, unknown>): Promise<void>;
  search(
    query: string,
    options?: { filters?: Record<string, string> },
  ): Promise<{ results: PagefindResult[] } | null>;
}

/** The sections a search may be limited to, by their address value. */
const SEARCH_SECTIONS: Readonly<Record<string, string>> = {
  reading: 'Reading',
  notes: 'Notes',
  diary: 'Diary',
  sketches: 'Sketches',
};

/** At most this many pages' records are read for one query. */
const SEARCH_PAGE_LIMIT = 50;
/** Results shown at first, and added by "More results". */
const SEARCH_BATCH = 20;

/** One thing found: a note, a sketch, a reading item, or a diary entry. */
interface Found {
  url: string;
  title: string;
  section: string;
  date: string | null;
  excerpt: string;
  hits: number;
}

let pagefindLoading: Promise<Pagefind | null> | null = null;

/** Pagefind's script, from the index built with the site; null when the
 * site has no index because nothing has been published to search yet. */
function loadPagefind(): Promise<Pagefind | null> {
  if (!pagefindLoading) {
    const address = '/pagefind/pagefind.js';
    pagefindLoading = (import(address) as Promise<Pagefind>)
      .then(async (pagefind) => {
        await pagefind.options({ excerptLength: 28 });
        return pagefind;
      })
      .catch(async (error: unknown) => {
        const answer = await fetch('/pagefind/pagefind-entry.json', { method: 'HEAD' }).catch(
          () => null,
        );
        if (answer && answer.status === 404) return null;
        pagefindLoading = null;
        throw error;
      });
  }
  return pagefindLoading;
}

const DIARY_ANCHOR = /^diary-([0-9]{4})-([0-9]{2})-([0-9]{2})(?:-[a-z0-9-]+)?$/;
// A work's anchor: its id when it has one, otherwise a hash of its title.
const READING_ANCHOR = /^reading-[a-z0-9]+(?:-[a-z0-9]+)*$/;
const MONTHS = [
  'January', 'February', 'March', 'April', 'May', 'June',
  'July', 'August', 'September', 'October', 'November', 'December',
];

function longDate(year: string, month: string, day: string): string {
  return `${Number(day)} ${MONTHS[Number(month) - 1] ?? month} ${year}`;
}

const withoutBreaks = (text: string): string => text.replace(/​/g, '').replace(/\s+/g, ' ').trim();

/** The entry an index position belongs to: the last matching anchor before it. */
function entryAnchor(data: PagefindData, location: number, pattern: RegExp): PagefindAnchor | null {
  let found: PagefindAnchor | null = null;
  for (const anchor of data.anchors) {
    if (anchor.location <= location && pattern.test(anchor.id)) {
      if (!found || anchor.location >= found.location) found = anchor;
    }
  }
  return found;
}

/** A page's record as things found: a Diary month or the Reading page is
 * split into the entries or works that matched; a note or sketch is one. */
function foundIn(data: PagefindData): Found[] {
  const section = data.filters.section?.[0] ?? '';
  const title = withoutBreaks(data.meta.title ?? '');
  const anchorPattern = section === 'Diary' ? DIARY_ANCHOR : section === 'Reading' ? READING_ANCHOR : null;
  if (!anchorPattern) {
    return [
      {
        url: data.url,
        title,

        section,
        date: data.meta.date ?? null,
        excerpt: data.excerpt,
        hits: data.sub_results.reduce((sum, sub) => sum + sub.locations.length, 0),
      },
    ];
  }
  const entries = new Map<string, Found>();
  for (const sub of data.sub_results) {
    const location = sub.locations[0] ?? sub.anchor?.location ?? 0;
    const anchor = entryAnchor(data, location, anchorPattern);
    if (!anchor) continue;
    const known = entries.get(anchor.id);
    if (known) {
      known.hits += sub.locations.length;
      continue;
    }
    let label = withoutBreaks(anchor.text);
    let date: string | null = null;
    const day = DIARY_ANCHOR.exec(anchor.id);
    if (day) {
      date = `${day[1]}-${day[2]}-${day[3]}`;
      const written = longDate(day[1]!, day[2]!, day[3]!);
      label = label ? `${written} — ${label}` : written;
    }
    entries.set(anchor.id, {
      url: `${data.url}#${anchor.id}`,
      title: label || title,
      section,
      date,
      excerpt: sub.excerpt,
      hits: sub.locations.length,
    });
  }
  const list = Array.from(entries.values()).sort((left, right) => right.hits - left.hits);
  return list.length > 0
    ? list
    : [{ url: data.url, title, section, date: null, excerpt: data.excerpt, hits: 0 }];
}

/** Pagefind's excerpt, keeping only its text and its <mark> highlights. */
function excerptNodes(excerpt: string): Node[] {
  const parsed = new DOMParser().parseFromString(`<p>${excerpt}</p>`, 'text/html');
  const nodes: Node[] = [];
  for (const node of Array.from(parsed.body.firstChild?.childNodes ?? [])) {
    const text = (node.textContent ?? '').replace(/​/g, '');
    if (node.nodeName === 'MARK') {
      const mark = document.createElement('mark');
      mark.textContent = text;
      nodes.push(mark);
    } else {
      nodes.push(document.createTextNode(text));
    }
  }
  return nodes;
}

function resultItem(found: Found): HTMLLIElement {
  const item = document.createElement('li');
  const heading = document.createElement('h2');
  const link = document.createElement('a');
  link.href = found.url;
  link.textContent = found.title || found.url;
  heading.append(link);
  const meta = document.createElement('p');
  meta.className = 'search-meta';
  meta.textContent = found.section;
  // A diary entry's title is its date already.
  if (found.section !== 'Diary' && found.date && /^[0-9]{4}-[0-9]{2}-[0-9]{2}$/.test(found.date)) {
    const [year, month, day] = found.date.split('-');
    const time = document.createElement('time');
    time.dateTime = found.date;
    time.textContent = longDate(year!, month!, day!);
    meta.append(' · ', time);
  }
  item.append(heading, meta);
  if (found.excerpt) {
    const excerpt = document.createElement('p');
    excerpt.className = 'search-excerpt';
    excerpt.append(...excerptNodes(found.excerpt));
    item.append(excerpt);
  }
  return item;
}

// The search page. Its form also works without the script: it reloads the
// page with ?q= and &section= in the address, which the script reads. With
// the script, results follow the typing, the address keeps the query and
// section (so a reload or a shared link shows the same search), and the
// status line distinguishes an empty query from a search with no results.
function setUpSearch(): void {
  const root = document.querySelector<HTMLElement>('[data-search-page]');
  if (!root) return;
  const form = root.querySelector<HTMLFormElement>('[data-search-form]');
  const input = root.querySelector<HTMLInputElement>('[data-search-query]');
  const status = root.querySelector<HTMLElement>('[data-search-status]');
  const list = root.querySelector<HTMLOListElement>('[data-search-results]');
  const more = root.querySelector<HTMLButtonElement>('[data-search-more]');
  if (!form || !input || !status || !list || !more) return;
  const scopes = Array.from(form.querySelectorAll<HTMLInputElement>('input[name="section"]'));

  const section = (): string => scopes.find((scope) => scope.checked)?.value ?? '';
  const read = (): void => {
    const params = new URLSearchParams(window.location.search);
    input.value = params.get('q') ?? '';
    const wanted = params.get('section') ?? '';
    const known = wanted in SEARCH_SECTIONS ? wanted : '';
    for (const scope of scopes) scope.checked = scope.value === known;
  };
  const write = (): void => {
    const params = new URLSearchParams();
    const query = input.value.trim();
    if (query) params.set('q', query);
    if (section()) params.set('section', section());
    const search = params.toString();
    const address = `${window.location.pathname}${search ? `?${search}` : ''}`;
    if (address !== `${window.location.pathname}${window.location.search}`) {
      window.history.replaceState(null, '', address);
    }
  };

  let found: Found[] = [];
  let shown = 0;
  let generation = 0;

  const showMore = (): void => {
    const next = found.slice(shown, shown + SEARCH_BATCH);
    list.append(...next.map(resultItem));
    shown += next.length;
    more.hidden = shown >= found.length;
  };

  const run = async (): Promise<void> => {
    const current = ++generation;
    const query = input.value.trim();
    const label = SEARCH_SECTIONS[section()];
    const where = label ? ` in ${label}` : '';
    if (!query) {
      found = [];
      list.replaceChildren();
      more.hidden = true;
      status.textContent = label
        ? `Enter a word or phrase to search ${label}.`
        : 'Enter a word or phrase to search Reading, Notes, Diary, and Sketches.';
      return;
    }
    let pagefind: Pagefind | null;
    try {
      pagefind = await loadPagefind();
    } catch {
      if (current !== generation) return;
      status.textContent = 'The search could not be loaded. Try again later.';
      return;
    }
    const answer = pagefind
      ? await pagefind.search(query, label ? { filters: { section: label } } : {})
      : { results: [] };
    if (!answer || current !== generation) return;
    const pages = await Promise.all(
      answer.results.slice(0, SEARCH_PAGE_LIMIT).map((result) => result.data()),
    );
    if (current !== generation) return;
    found = pages.flatMap(foundIn);
    shown = 0;
    list.replaceChildren();
    if (found.length === 0) {
      status.textContent = 'No matches.';
      if (label) {
        const everywhere = document.createElement('a');
        const params = new URLSearchParams({ q: query });
        everywhere.href = `${window.location.pathname}?${params.toString()}`;
        everywhere.textContent = 'Search all sections';
        everywhere.addEventListener('click', (event) => {
          event.preventDefault();
          for (const scope of scopes) scope.checked = scope.value === '';
          write();
          void run();
        });
        status.append(' ', everywhere);
      }
      more.hidden = true;
      return;
    }
    const capped = answer.results.length > SEARCH_PAGE_LIMIT ? '+' : '';
    status.textContent = `${found.length}${capped} ${found.length === 1 && !capped ? 'result' : 'results'} for “${query}”${where}.`;
    showMore();
  };

  let timer: number | undefined;
  input.addEventListener('input', () => {
    window.clearTimeout(timer);
    timer = window.setTimeout(() => {
      write();
      void run();
    }, 220);
  });
  for (const scope of scopes) {
    scope.addEventListener('change', () => {
      write();
      void run();
    });
  }
  form.addEventListener('submit', (event) => {
    event.preventDefault();
    window.clearTimeout(timer);
    write();
    void run();
  });
  more.addEventListener('click', () => {
    const first = shown;
    showMore();
    list.children[first]?.querySelector('a')?.focus();
  });
  window.addEventListener('popstate', () => {
    read();
    void run();
  });

  read();
  void run();
  if (!input.value) input.focus();
}

// ---------------------------------------------------------------- music cards

/*
 * A music card (site-builder/src/music.rs). The services are a choice that
 * plays nothing; only "Play here" makes the chosen service's own player,
 * below the card, and one player is on the page at a time: opening
 * another, closing it, or choosing the other service removes it (and so
 * its sound). Nothing starts by itself: the services' players start when
 * their own play button is pressed. Only the two services' player
 * addresses are accepted.
 */
const PLAYERS: Record<string, { name: string; pattern: RegExp }> = {
  'apple-music': {
    name: 'Apple Music',
    pattern: /^https:\/\/embed\.music\.apple\.com\/[a-z]{2}\/(?:album|song|playlist)\/(?:[A-Za-z0-9._~%-]+\/)?(?:\d+|pl\.[A-Za-z0-9-]+)(?:\?i=\d+)?$/,
  },
  spotify: {
    name: 'Spotify',
    pattern: /^https:\/\/open\.spotify\.com\/embed\/(?:track|album|playlist)\/[A-Za-z0-9]{22}$/,
  },
};

/** The one player on the page, if any. */
let openPlayer: { close: () => void } | null = null;
let playerCount = 0;
/** Places each switch's sliding pill again when its services change size (fonts, width). */
const placeOnResize = new Map<Element, () => void>();
let switchObserver: ResizeObserver | null = null;

function setUpMusicCards(): void {
  for (const card of document.querySelectorAll<HTMLElement>('[data-music-card]')) {
    const options = Array.from(card.querySelectorAll<HTMLAnchorElement>('a.music-option'));
    if (options.length === 0 || !options.every((o) => PLAYERS[o.dataset.service ?? '']?.pattern.test(o.dataset.embed ?? ''))) continue;
    const toggle = card.querySelector<HTMLElement>('.music-switch');
    const play = card.querySelector<HTMLButtonElement>('.music-play');
    const open = card.querySelector<HTMLAnchorElement>('.music-open');
    const label = card.querySelector<HTMLElement>('.music-play-label');
    if (!toggle || !play || !open || !label) continue;
    const title = card.querySelector('.music-title')?.textContent ?? '';
    const playerId = `music-player-${++playerCount}`;
    let chosen = 0;
    let player: HTMLElement | null = null;

    const name = () => PLAYERS[options[chosen]!.dataset.service!]!.name;
    const close = (): void => {
      player?.remove();
      player = null;
      play.setAttribute('aria-expanded', 'false');
      play.setAttribute('aria-label', `Play here: open the ${name()} player`);
      label.textContent = 'Play here';
      if (openPlayer?.close === close) openPlayer = null;
    };
    // The pill under the chosen service, as wide as it: the two may differ.
    const place = (): void => {
      const option = options[chosen]!;
      toggle.style.setProperty('--music-x', `${option.offsetLeft}px`);
      toggle.style.setProperty('--music-w', `${option.offsetWidth}px`);
      toggle.dataset.placed = '';
    };
    const choose = (n: number): void => {
      if (n === chosen) return;
      chosen = n;
      // Another service: the player of the one before goes (with its sound), none starts.
      if (player) close();
      place();
      options.forEach((o, i) => {
        o.setAttribute('aria-checked', String(i === n));
        o.tabIndex = i === n ? 0 : -1;
      });
      open.href = options[n]!.href;
      open.setAttribute('aria-label', `Open in ${name()}`);
      play.setAttribute('aria-label', `Play here: open the ${name()} player`);
    };

    if (options.length > 1) {
      switchObserver ??= new ResizeObserver((entries) => {
        for (const entry of entries) placeOnResize.get(entry.target)?.();
      });
      for (const option of options) {
        placeOnResize.set(option, place);
        switchObserver.observe(option);
      }
      toggle.setAttribute('role', 'radiogroup');
      toggle.setAttribute('aria-label', `Service for ${title}`);
      options.forEach((option, i) => {
        option.removeAttribute('aria-current');
        option.setAttribute('role', 'radio');
        option.setAttribute('aria-checked', String(i === 0));
        option.tabIndex = i === 0 ? 0 : -1;
        option.addEventListener('click', (event) => {
          // A modified click still opens the service's page, as a link would.
          if (event.metaKey || event.ctrlKey || event.shiftKey || event.altKey || event.button !== 0) return;
          event.preventDefault();
          choose(i);
        });
        option.addEventListener('keydown', (event) => {
          const step = event.key === 'ArrowRight' || event.key === 'ArrowDown' ? 1 : event.key === 'ArrowLeft' || event.key === 'ArrowUp' ? -1 : 0;
          if (step) {
            event.preventDefault();
            const next = (i + step + options.length) % options.length;
            choose(next);
            options[next]!.focus();
          } else if (event.key === ' ') {
            event.preventDefault();
            choose(i);
          }
        });
      });
    } else {
      // One service: its name, not a choice.
      const only = options[0]!;
      only.removeAttribute('href');
      only.removeAttribute('aria-current');
    }

    play.disabled = false;
    play.setAttribute('aria-controls', playerId);
    play.setAttribute('aria-expanded', 'false');
    play.setAttribute('aria-label', `Play here: open the ${name()} player`);
    play.addEventListener('click', () => {
      if (player) {
        close();
        return;
      }
      openPlayer?.close();
      const option = options[chosen]!;
      const box = document.createElement('div');
      box.className = 'music-player';
      box.id = playerId;
      const status = document.createElement('p');
      status.className = 'music-player-status';
      status.textContent = `Loading the ${name()} player…`;
      const frame = document.createElement('iframe');
      frame.src = option.dataset.embed!;
      frame.title = `${name()} player: ${title}`;
      frame.height = String(Math.min(500, Math.max(80, Number(option.dataset.height) || 175)));
      frame.referrerPolicy = 'strict-origin-when-cross-origin';
      frame.allow = 'autoplay; encrypted-media; fullscreen; clipboard-write';
      frame.setAttribute(
        'sandbox',
        'allow-scripts allow-same-origin allow-popups allow-popups-to-escape-sandbox allow-storage-access-by-user-activation allow-top-navigation-by-user-activation',
      );
      const waiting = window.setTimeout(() => {
        status.textContent = `The ${name()} player has not loaded. Use ↗ to open ${name()}.`;
      }, 15000);
      frame.addEventListener('load', () => {
        window.clearTimeout(waiting);
        status.textContent = '';
      });
      box.append(status, frame);
      card.append(box);
      player = box;
      openPlayer = { close };
      play.setAttribute('aria-expanded', 'true');
      play.setAttribute('aria-label', `Close the ${name()} player`);
      label.textContent = 'Close';
    });
    card.classList.add('music-ready');
  }
}

// ---------------------------------------------------------------- reading state

/*
 * Which Miscellany articles this visitor has opened, and which they have
 * marked as read, so that lists can tell them apart. The record exists only
 * in this browser's localStorage: it is never sent anywhere, and nothing else
 * on the site depends on it. When storage cannot be used, or the record
 * cannot be read, nothing is shown and nothing is written, so a damaged
 * record is never overwritten and the site works as if the feature were
 * absent.
 *
 * An article is known by a key made of its section and its stable identity
 * (site-builder/src/render.rs, `reading_key`): `notes/<slug>`,
 * `sketches/<slug>`, `diary/<date>[-<slug>]`, `reading/<id or hash>`. The
 * same key is derived from any link to the article, so a note opened from
 * Recent Updates is marked in the Notes list and in search results too.
 *
 * Opening a note's or sketch's page records it as visited. Diary entries and
 * reading records share a page with others, so one of them counts as visited
 * once it has been on screen for a moment, or when the page is opened at its
 * own anchor; opening a section's page never marks everything in it. An
 * article is "read" only when the visitor says so, and "Mark as unread" makes
 * it visited again.
 */

type ReadingState = 'visited' | 'read';

const READING_STORE = 'miscellany-reading';
const READING_VERSION = 1;
const SLUG = '[a-z0-9]+(?:-[a-z0-9]+)*';
const ARTICLE_PAGE = new RegExp(`^/miscellany/(notes|sketches)/(${SLUG})/$`);
const DIARY_PAGE = /^\/miscellany\/diary\/(?:[0-9]{4}\/[0-9]{2}\/)?$/;
const DIARY_KEY = new RegExp(`^#diary-([0-9]{4}-[0-9]{2}-[0-9]{2}(?:-${SLUG})?)$`);
const READING_KEY = new RegExp(`^#reading-(${SLUG})$`);
/** How long a diary entry or reading record stays on screen to count as seen. */
const SEEN_AFTER_MS = 1200;

/** The article a link leads to, or null for anything that is not one. */
function articleKey(href: string): string | null {
  let url: URL;
  try {
    url = new URL(href, window.location.href);
  } catch {
    return null;
  }
  if (url.origin !== window.location.origin) return null;
  const page = ARTICLE_PAGE.exec(url.pathname);
  if (page) return `${page[1]}/${page[2]}`;
  const diary = DIARY_PAGE.test(url.pathname) ? DIARY_KEY.exec(url.hash) : null;
  if (diary) return `diary/${diary[1]}`;
  const reading = url.pathname === '/miscellany/reading/' ? READING_KEY.exec(url.hash) : null;
  if (reading) return `reading/${reading[1]}`;
  return null;
}

/** The stored record, or null when storage is unavailable or the record is
 * not one this script wrote. An absent record is an empty one. */
function readReading(): Map<string, ReadingState> | null {
  let raw: string | null;
  try {
    raw = window.localStorage.getItem(READING_STORE);
  } catch {
    return null;
  }
  const states = new Map<string, ReadingState>();
  if (raw === null) return states;
  try {
    const parsed: unknown = JSON.parse(raw);
    if (typeof parsed !== 'object' || parsed === null) return null;
    const { version, articles } = parsed as { version?: unknown; articles?: unknown };
    if (version !== READING_VERSION || typeof articles !== 'object' || articles === null) return null;
    for (const [key, state] of Object.entries(articles)) {
      if (state === 'visited' || state === 'read') states.set(key, state);
    }
  } catch {
    return null;
  }
  return states;
}

/** Changes one article's state, reading the record afresh first so that
 * another tab's changes are kept. Returns the record as written, or null. */
function updateReading(key: string, change: (state: ReadingState | undefined) => ReadingState): Map<string, ReadingState> | null {
  const states = readReading();
  if (!states) return null;
  const next = change(states.get(key));
  if (states.get(key) === next) return states;
  states.set(key, next);
  try {
    window.localStorage.setItem(
      READING_STORE,
      JSON.stringify({ version: READING_VERSION, articles: Object.fromEntries(states) }),
    );
  } catch {
    return null;
  }
  return states;
}

const visit = (key: string): Map<string, ReadingState> | null =>
  updateReading(key, (state) => state ?? 'visited');

const MARK_TEXT: Record<string, string> = {
  unvisited: ', not yet opened',
  visited: '',
  read: ', read',
};

/** Shows one state beside a title: a small dot for an article not yet
 * opened, a quieter title for one opened, a small check for one read. The
 * mark takes no room, so nothing moves when it appears, and it carries the
 * same words for screen readers. */
function markTarget(target: HTMLElement, state: 'unvisited' | ReadingState | null): void {
  let mark = target.querySelector<HTMLElement>(':scope > .reading-mark');
  if (state === null) {
    mark?.remove();
    delete target.dataset.readingState;
    return;
  }
  if (!mark) {
    mark = document.createElement('span');
    mark.className = 'reading-mark';
    const words = document.createElement('span');
    words.className = 'sr-only';
    mark.append(words);
    target.append(mark);
  }
  mark.dataset.state = state;
  mark.firstElementChild!.textContent = MARK_TEXT[state] ?? '';
  target.dataset.readingState = state;
}

/** The state shown for several articles together (a Diary month): not yet
 * opened while any is, read when all are, and otherwise opened. */
function combined(states: Map<string, ReadingState>, keys: string[]): 'unvisited' | ReadingState {
  const each = keys.map((key) => states.get(key));
  if (each.some((state) => state === undefined)) return 'unvisited';
  return each.every((state) => state === 'read') ? 'read' : 'visited';
}

function applyReading(states: Map<string, ReadingState> | null): void {
  // Marks appear once the visitor has opened something: on a first visit
  // everything would be "not yet opened", which says nothing.
  const showing = states !== null && states.size > 0;
  const stateOf = (key: string): 'unvisited' | ReadingState | null =>
    showing ? states!.get(key) ?? 'unvisited' : null;

  for (const link of document.querySelectorAll<HTMLAnchorElement>('[data-reading-list] a[href]')) {
    const key = articleKey(link.getAttribute('href') ?? '');
    if (key) markTarget(link, stateOf(key));
  }
  // In Updates an article's title is inside a sentence (Published “…” in
  // Notes.), so its mark goes at the end of the sentence, clear of the
  // quotation mark after the title. Other links there are not articles.
  for (const link of document.querySelectorAll<HTMLAnchorElement>('.update-list h3 a[href], .recent-updates p a[href]')) {
    const key = articleKey(link.getAttribute('href') ?? '');
    if (key && link.parentElement) markTarget(link.parentElement, stateOf(key));
  }
  // A Diary month: its mark goes after the row's entry count (the link's own
  // label is the month's name, which would hide words placed inside it).
  for (const link of document.querySelectorAll<HTMLAnchorElement>('a[data-reading-entries]')) {
    const keys = (link.dataset.readingEntries ?? '').split(' ').filter(Boolean);
    if (link.parentElement) markTarget(link.parentElement, showing && keys.length ? combined(states!, keys) : null);
  }
  for (const article of document.querySelectorAll<HTMLElement>('[data-reading-article]')) {
    const key = article.dataset.readingArticle ?? '';
    const target = article.querySelector<HTMLElement>('[data-reading-target]');
    if (target) markTarget(target, stateOf(key));
  }
  for (const control of document.querySelectorAll<HTMLElement>('[data-reading-control]')) {
    const article = control.closest<HTMLElement>('[data-reading-article]')
      ?? document.querySelector<HTMLElement>('article[data-reading-article]');
    const button = control.querySelector('button');
    if (!article || !button || states === null) {
      control.hidden = true;
      continue;
    }
    const read = states.get(article.dataset.readingArticle ?? '') === 'read';
    button.setAttribute('aria-pressed', String(read));
    button.textContent = read ? 'Mark as unread' : 'Mark as read';
    control.dataset.read = String(read);
    control.hidden = false;
  }
}

function setUpReadingState(): void {
  if (readReading() === null) return;

  // A note's or sketch's own page: opening it is a visit.
  const own = document.querySelector<HTMLElement>('article.miscellany-note[data-reading-article]');
  if (own?.dataset.readingArticle) visit(own.dataset.readingArticle);

  // Arriving at a diary entry's or reading record's anchor is a visit too.
  const arrived = articleKey(window.location.href);
  if (arrived) visit(arrived);

  // Entries that share a page count as seen after a moment on screen.
  const seen = document.querySelectorAll<HTMLElement>('[data-reading-article][data-reading-seen]');
  if (seen.length && 'IntersectionObserver' in window) {
    const timers = new Map<Element, number>();
    const observer = new IntersectionObserver((records) => {
      for (const record of records) {
        const element = record.target as HTMLElement;
        const visible = record.isIntersecting
          && (record.intersectionRatio >= 0.5 || record.intersectionRect.height >= window.innerHeight * 0.5);
        if (visible && !timers.has(element)) {
          timers.set(element, window.setTimeout(() => {
            observer.unobserve(element);
            applyReading(visit(element.dataset.readingArticle ?? ''));
          }, SEEN_AFTER_MS));
        } else if (!visible && timers.has(element)) {
          window.clearTimeout(timers.get(element));
          timers.delete(element);
        }
      }
    }, { threshold: [0, 0.5, 1] });
    for (const element of seen) observer.observe(element);
  }

  for (const control of document.querySelectorAll<HTMLElement>('[data-reading-control]')) {
    control.querySelector('button')?.addEventListener('click', () => {
      const article = control.closest<HTMLElement>('[data-reading-article]')
        ?? document.querySelector<HTMLElement>('article[data-reading-article]');
      const key = article?.dataset.readingArticle;
      if (!key) return;
      applyReading(updateReading(key, (state) => (state === 'read' ? 'visited' : 'read')));
    });
  }

  // Search results arrive after the page; mark them as they do.
  for (const list of document.querySelectorAll('[data-reading-list]')) {
    new MutationObserver(() => applyReading(readReading())).observe(list, { childList: true });
  }
  // Another tab's change, or the record cleared.
  window.addEventListener('storage', (event) => {
    if (event.key === READING_STORE || event.key === null) applyReading(readReading());
  });
  // Returning to a list with the back button restores it from the cache.
  window.addEventListener('pageshow', (event) => {
    if (event.persisted) applyReading(readReading());
  });

  applyReading(readReading());
}

// ------------------------------------------------------------ a message to the author

/*
 * The form at the end of Reading, Notes, Diary, and Sketches
 * (site-builder/src/render.rs message_form): sent to the Writing Studio's
 * intake, which keeps it for the author alone. Nothing sent is shown on the
 * site. The form is drawn hidden and shown here, so a page without scripts
 * offers nothing that cannot work.
 */
const MESSAGE_ERRORS: Readonly<Record<string, string>> = {
  empty: 'Write a message first.',
  too_long: 'The message is longer than 4,000 characters.',
  too_many_links: 'Please include no more than three links.',
  name_too_long: 'The name is longer than 80 characters.',
  unknown_article: 'One of the chosen articles is no longer published; reload the page and choose again.',
  too_many: 'Several messages were sent from here just now. Please try again later.',
  busy: 'Many messages have arrived today. Please try again tomorrow.',
};

function setUpSiteMessage(): void {
  const section = document.querySelector<HTMLElement>('[data-site-message]');
  const form = section?.querySelector<HTMLFormElement>('[data-site-message-form]');
  if (!section || !form) return;
  const endpoint = section.dataset.endpoint ?? '';
  const category = section.dataset.category ?? '';
  const status = form.querySelector<HTMLElement>('.site-message-status');
  const button = form.querySelector<HTMLButtonElement>('button[type="submit"]');
  const body = form.querySelector<HTMLTextAreaElement>('textarea[name="body"]');
  if (!endpoint || !status || !button || !body) return;
  section.hidden = false;
  const shownAt = Date.now();
  // Three lines to start with; taller as the message grows (to about half a screen, then it scrolls).
  const grow = () => {
    body.style.height = 'auto';
    body.style.height = `${Math.min(body.scrollHeight + 2, Math.max(window.innerHeight * 0.5, 160))}px`;
  };
  body.addEventListener('input', grow);
  const say = (text: string, kind: 'done' | 'problem' | '') => {
    status.textContent = text;
    status.dataset.kind = kind;
  };
  form.addEventListener('submit', async (event) => {
    event.preventDefault();
    const text = body.value.trim();
    if (!text) {
      say(MESSAGE_ERRORS.empty!, 'problem');
      body.focus();
      return;
    }
    const data = new FormData(form);
    const payload = {
      category,
      body: text,
      name: String(data.get('name') ?? ''),
      articles: data.getAll('articles').map(String),
      website: String(data.get('website') ?? ''),
      elapsed: Date.now() - shownAt,
    };
    button.disabled = true;
    say('Sending…', '');
    try {
      const response = await fetch(endpoint, { method: 'POST', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify(payload), credentials: 'omit', mode: 'cors' });
      if (response.status === 202) {
        form.reset();
        grow();
        say('Sent. Thank you.', 'done');
        return;
      }
      const answer = (await response.json().catch(() => ({}))) as { error?: string };
      say(MESSAGE_ERRORS[answer.error ?? ''] ?? 'The message could not be sent just now. Please try again later.', 'problem');
    } catch {
      say('The message could not be sent (no connection?). It is still here; try again.', 'problem');
    } finally {
      button.disabled = false;
    }
  });
}
