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

  if (button) {
    const label = resolved === 'dark' ? button.dataset.lightLabel : button.dataset.darkLabel;
    if (label) {
      button.setAttribute('aria-label', label);
      button.title = label;
    }
  }
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
