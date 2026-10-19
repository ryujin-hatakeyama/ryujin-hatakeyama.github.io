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
const READING_ANCHOR = /^reading-[0-9a-f]{8}(?:-[0-9]+)?$/;
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
      status.textContent = `No results for “${query}”${where}.`;
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
