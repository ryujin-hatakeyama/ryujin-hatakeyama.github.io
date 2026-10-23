/*
 * The section list beside a long page ("On this page"): marks the link of the
 * section being read. The list itself, and the headings' identifiers, are
 * written by the builder; this only sets aria-current on one link.
 *
 * A section is current from the moment its heading crosses a line a quarter
 * of the way down the window until the next heading does. The last section
 * becomes current when the page ends.
 */
export function setUpSectionNav(): void {
  const nav = document.querySelector<HTMLElement>('[data-section-nav]');
  if (!nav || typeof IntersectionObserver === 'undefined') return;

  const links = Array.from(nav.querySelectorAll<HTMLAnchorElement>('a[href^="#"]'));
  const sections: { heading: HTMLElement; link: HTMLAnchorElement }[] = [];
  for (const link of links) {
    let id = link.getAttribute('href')!.slice(1);
    try {
      id = decodeURIComponent(id);
    } catch {
      // An id that is not percent-encoded is used as it stands.
    }
    const heading = document.getElementById(id);
    if (heading) sections.push({ heading, link });
  }
  if (sections.length === 0) return;

  let current: HTMLAnchorElement | null = null;
  const mark = (link: HTMLAnchorElement | null): void => {
    if (link === current) return;
    current?.removeAttribute('aria-current');
    link?.setAttribute('aria-current', 'location');
    current = link;
  };

  // The current section is the last whose heading is above the reading line.
  const above = new Set<HTMLElement>();
  const settle = (): void => {
    const atEnd = window.innerHeight + window.scrollY >= document.documentElement.scrollHeight - 2;
    if (atEnd) return mark(sections[sections.length - 1].link);
    let found: HTMLAnchorElement | null = null;
    for (const section of sections) {
      if (above.has(section.heading)) found = section.link;
    }
    mark(found);
  };

  // Rows are observed from the top of the window down to the reading line
  // (the upper quarter); a heading is "above" while it is in that band or has
  // left through the top, which the bounding box tells apart on each change.
  const observer = new IntersectionObserver(
    (entries) => {
      for (const entry of entries) {
        const heading = entry.target as HTMLElement;
        if (entry.isIntersecting || entry.boundingClientRect.top < 0) above.add(heading);
        else above.delete(heading);
      }
      settle();
    },
    { rootMargin: '0px 0px -75% 0px' },
  );
  for (const section of sections) observer.observe(section.heading);

  // The end of the page is not a heading crossing a line.
  const sentinel = document.createElement('div');
  sentinel.setAttribute('aria-hidden', 'true');
  sentinel.style.cssText = 'height:1px;margin-top:-1px;pointer-events:none';
  document.body.append(sentinel);
  new IntersectionObserver(() => settle()).observe(sentinel);

  // A heading in the address is current at once, before any scrolling.
  const settleFromHash = (): void => {
    const target = sections.find(({ link }) => link.hash === window.location.hash);
    if (target) mark(target.link);
  };
  window.addEventListener('hashchange', settleFromHash);
  settleFromHash();
}
