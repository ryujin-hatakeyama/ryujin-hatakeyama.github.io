/*
 * Media cards (site-builder/src/media/), loaded by site.ts only on a page
 * with a card that offers two services. The services become one choice, and
 * "Open" opens the chosen one; choosing never navigates. A podcast's player
 * follows the choice: the build keeps each other service's player inert in
 * a <template>, and the chosen one replaces the one before (and so stops its
 * sound), so a card never has two. Nothing plays until its own play button
 * is pressed. The addresses are the build's, checked there.
 */

/** Places each switch's sliding pill again when its services change size (fonts, width). */
const placeOnResize = new Map<Element, () => void>();
let switchObserver: ResizeObserver | null = null;

export function setUpMediaCards(): void {
  for (const card of document.querySelectorAll<HTMLElement>('[data-media-card]')) {
    const toggle = card.querySelector<HTMLElement>('.media-switch');
    const open = card.querySelector<HTMLAnchorElement>('.media-open');
    const service = open?.querySelector<HTMLElement>('[data-media-open-service]');
    const options = Array.from(toggle?.querySelectorAll<HTMLAnchorElement>('a.media-option') ?? []);
    if (!toggle || !open || !service || options.length < 2) continue;
    const player = card.querySelector<HTMLElement>('.media-player');
    const live = player?.querySelector<HTMLIFrameElement>(':scope > iframe');
    if (player && live) {
      // The first player, kept like the others, to come back to.
      const kept = document.createElement('template');
      kept.dataset.service = live.dataset.service;
      kept.content.append(live.cloneNode());
      player.append(kept);
    }
    const title = card.querySelector('.media-title')?.textContent ?? '';
    let chosen = 0;

    // The pill under the chosen service, as wide as it: the two may differ.
    const place = (): void => {
      const option = options[chosen]!;
      toggle.style.setProperty('--media-x', `${option.offsetLeft}px`);
      toggle.style.setProperty('--media-w', `${option.offsetWidth}px`);
      toggle.dataset.placed = '';
    };
    const choose = (n: number): void => {
      if (n === chosen) return;
      chosen = n;
      const option = options[n]!;
      options.forEach((o, i) => {
        o.setAttribute('aria-checked', String(i === n));
        o.tabIndex = i === n ? 0 : -1;
      });
      place();
      open.href = option.href;
      service.textContent = ` in ${option.textContent ?? ''}`;
      const frame = player?.querySelector(':scope > iframe');
      const next = player?.querySelector<HTMLTemplateElement>(
        `:scope > template[data-service="${option.dataset.service ?? ''}"]`,
      );
      const fresh = next && document.importNode(next.content, true).firstElementChild;
      if (frame && fresh) frame.replaceWith(fresh);
    };

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
        const step =
          event.key === 'ArrowRight' || event.key === 'ArrowDown'
            ? 1
            : event.key === 'ArrowLeft' || event.key === 'ArrowUp'
              ? -1
              : 0;
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
  }
}
