function hashAnchor(): string | null {
  if (!window.location.hash.startsWith('#')) return null;
  try {
    return decodeURIComponent(window.location.hash.slice(1));
  } catch {
    return window.location.hash.slice(1);
  }
}

export function setUpDiaryWeeks(): void {
  for (const navigator of document.querySelectorAll<HTMLElement>('[data-diary-weeks]')) {
    const weeks = Array.from(navigator.querySelectorAll<HTMLElement>('[data-diary-week]'));
    const previous = navigator.querySelector<HTMLButtonElement>('[data-diary-week-previous]');
    const next = navigator.querySelector<HTMLButtonElement>('[data-diary-week-next]');
    if (weeks.length === 0 || !previous || !next) continue;

    let current = Math.max(0, weeks.findIndex((week) => !week.hidden));

    const show = (index: number): void => {
      current = Math.max(0, Math.min(index, weeks.length - 1));
      weeks.forEach((week, position) => { week.hidden = position !== current; });
      previous.disabled = current === 0;
      next.disabled = current === weeks.length - 1;
    };

    const selectHash = (): void => {
      const anchor = hashAnchor();
      const links = Array.from(navigator.querySelectorAll<HTMLAnchorElement>('[data-diary-anchors]'));
      const selected = anchor
        ? links.find((link) => link.dataset.diaryAnchors?.split(' ').includes(anchor))
        : links.find((link) => link.dataset.diaryDate === navigator.dataset.defaultDate);
      for (const link of links) {
        if (link === selected) link.setAttribute('aria-current', 'date');
        else link.removeAttribute('aria-current');
      }
      const week = selected?.closest<HTMLElement>('[data-diary-week]');
      const index = week ? weeks.indexOf(week) : -1;
      if (index >= 0) show(index);
    };

    previous.hidden = false;
    next.hidden = false;
    previous.addEventListener('click', () => show(current - 1));
    next.addEventListener('click', () => show(current + 1));
    window.addEventListener('hashchange', selectHash);
    selectHash();
  }
}
