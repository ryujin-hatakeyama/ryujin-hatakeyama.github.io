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
