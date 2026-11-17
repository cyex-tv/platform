// Docs behaviour: scroll-spy, an index that follows the reader both ways, independent column
// scrolling, the Ctrl/Cmd+K filter and shared code-sample tabs. Call from an $effect once the
// sections have rendered; it returns its cleanup.

const HEADER_OFFSET = 80; // matches the sections' scroll-margin-top
const BAND = [0.25, 0.6]; // the comfortable zone of the index, as fractions of its height
const PARK = 0.35; // where an entry is brought to when it leaves the zone
const LANG_KEY = 'cyexDocsLang';

export function initDocs(root: HTMLElement): () => void {
  const aside = root.querySelector<HTMLElement>('.docs-nav')!;
  const nav = root.querySelector<HTMLElement>('.nav-scroll')!;
  const search = root.querySelector<HTMLElement>('.nav-search')!;
  const input = search.querySelector('input')!;
  const sections = [...root.querySelectorAll<HTMLElement>('section[id]')];
  const items = new Map([...nav.querySelectorAll<HTMLAnchorElement>('a.nav-item')].map((a) => [a.getAttribute('href')!.slice(1), a]));
  const reduced = () => matchMedia('(prefers-reduced-motion: reduce)').matches;
  const cleanups: (() => void)[] = [];
  const on = <K extends keyof WindowEventMap>(
    target: EventTarget,
    type: K | string,
    fn: (e: any) => void, // eslint-disable-line @typescript-eslint/no-explicit-any
    options?: AddEventListenerOptions,
  ) => {
    target.addEventListener(type, fn, options);
    cleanups.push(() => target.removeEventListener(type, fn, options));
  };

  // ---------- scroll-spy ----------
  let tops: { id: string; top: number }[] = [];
  const measure = () => {
    tops = sections.map((s) => ({ id: s.id, top: s.getBoundingClientRect().top + scrollY }));
  };

  let activeId: string | null = null;
  let edge: 'top' | 'end' | null = null;
  let frame = 0;

  function update() {
    frame = 0;
    if (!tops.length) return;
    const atTop = scrollY <= 2;
    const atEnd = innerHeight + scrollY >= document.documentElement.scrollHeight - 2;
    let current = tops[0];
    if (atEnd) current = tops[tops.length - 1];
    else {
      const y = scrollY + HEADER_OFFSET;
      for (const t of tops) {
        if (t.top <= y + 2) current = t;
        else break;
      }
    }
    const nextEdge = atTop ? 'top' : atEnd ? 'end' : null;
    if (current.id !== activeId || nextEdge !== edge) {
      setActive(current.id);
      edge = nextEdge;
      follow();
    }
  }
  const onScroll = () => {
    if (!frame) frame = requestAnimationFrame(update);
  };
  const onResize = () => {
    measure();
    onScroll();
  };

  function setActive(id: string) {
    if (id === activeId) return;
    if (activeId) {
      items.get(activeId)?.classList.remove('active');
      items.get(activeId)?.removeAttribute('aria-current');
    }
    activeId = id;
    items.get(id)?.classList.add('active');
    items.get(id)?.setAttribute('aria-current', 'location');
  }

  // ---------- the index follows the reader, both ways ----------
  let railTouchedAt = 0;

  function follow() {
    if (getComputedStyle(aside).position !== 'sticky') return;
    if (performance.now() - railTouchedAt < 1200) return;
    const max = nav.scrollHeight - nav.clientHeight;
    if (max <= 0) return;

    let target: number;
    if (edge === 'top') target = 0;
    else if (edge === 'end') target = max;
    else {
      const el = activeId ? items.get(activeId) : undefined;
      if (!el || el.hidden) return;
      const group = el.closest('.nav-group');
      const leads = group && group.querySelector('.nav-item:not([hidden])') === el;
      const anchor = (leads && group.querySelector('.nav-group-title')) || el;
      const box = nav.getBoundingClientRect();
      const h = nav.clientHeight;
      const top = anchor.getBoundingClientRect().top - box.top;
      const bottom = el.getBoundingClientRect().bottom - box.top;
      if (top >= h * BAND[0] && bottom <= h * BAND[1]) return;
      target = nav.scrollTop + top - h * PARK;
    }
    target = Math.max(0, Math.min(max, Math.round(target)));
    if (Math.abs(target - nav.scrollTop) < 2) return;
    nav.scrollTo({ top: target, behavior: reduced() ? 'auto' : 'smooth' });
  }

  // ---------- two columns, two scrolls ----------
  on(
    aside,
    'wheel',
    (e: WheelEvent) => {
      if (getComputedStyle(aside).position !== 'sticky') return;
      railTouchedAt = performance.now();
      if (!nav.contains(e.target as Node)) {
        e.preventDefault();
        return;
      }
      const atTop = nav.scrollTop <= 0;
      const atEnd = nav.scrollTop + nav.clientHeight >= nav.scrollHeight - 1;
      if ((e.deltaY < 0 && atTop) || (e.deltaY > 0 && atEnd)) e.preventDefault();
    },
    { passive: false },
  );
  on(nav, 'pointerdown', () => (railTouchedAt = performance.now()));
  on(nav, 'touchmove', () => (railTouchedAt = performance.now()), { passive: true });
  on(nav, 'click', (e: MouseEvent) => {
    if ((e.target as HTMLElement).closest('a.nav-item')) railTouchedAt = 0;
  });

  // ---------- filter ----------
  const kbd = search.querySelector('.nav-kbd');
  if (kbd) kbd.textContent = /Mac|iP(hone|ad|od)/.test(navigator.platform || navigator.userAgent) ? '⌘K' : 'Ctrl K';
  const empty = nav.querySelector<HTMLElement>('.nav-empty');
  const apply = () => {
    const q = input.value.trim().toLowerCase();
    search.classList.toggle('has-value', !!input.value);
    let any = false;
    for (const group of nav.querySelectorAll<HTMLElement>('.nav-group')) {
      let shown = 0;
      for (const a of group.querySelectorAll<HTMLElement>('.nav-item')) {
        const hit = !q || (a.textContent + ' ' + (a.dataset.search ?? '')).toLowerCase().includes(q);
        a.hidden = !hit;
        if (hit) shown++;
      }
      group.hidden = !shown;
      if (shown) any = true;
    }
    if (empty) {
      empty.hidden = any;
      empty.textContent = `Nothing matches “${input.value.trim()}”.`;
    }
  };
  on(input, 'input', apply);
  on(search.querySelector('.input-clear')!, 'click', () => {
    input.value = '';
    apply();
    input.focus();
  });
  on(document, 'keydown', (e: KeyboardEvent) => {
    if ((e.metaKey || e.ctrlKey) && e.key.toLowerCase() === 'k') {
      e.preventDefault();
      input.focus();
      input.select();
    } else if (e.key === 'Escape' && document.activeElement === input) {
      input.value = '';
      apply();
      input.blur();
    }
  });

  // ---------- code samples ----------
  const setLang = (lang: string) => {
    for (const tab of root.querySelectorAll<HTMLElement>('.lang-tab')) tab.setAttribute('aria-selected', String(tab.dataset.lang === lang));
    for (const pre of root.querySelectorAll<HTMLElement>('pre[data-lang]')) pre.hidden = pre.dataset.lang !== lang;
  };
  let lang = root.querySelector<HTMLElement>('.lang-tab')?.dataset.lang;
  try {
    const saved = localStorage.getItem(LANG_KEY);
    if (saved && root.querySelector(`.lang-tab[data-lang="${saved}"]`)) lang = saved;
  } catch {
    /* storage can be blocked; the first language stays */
  }
  if (lang) setLang(lang);
  on(root, 'click', (e: MouseEvent) => {
    const target = e.target as HTMLElement;
    const tab = target.closest<HTMLElement>('.lang-tab');
    if (tab) {
      setLang(tab.dataset.lang!);
      try {
        localStorage.setItem(LANG_KEY, tab.dataset.lang!);
      } catch {
        /* ignore */
      }
      return;
    }
    const copy = target.closest<HTMLElement>('.code-copy');
    if (!copy) return;
    const pre = [...copy.closest('.code-block')!.querySelectorAll('pre')].find((p) => !p.hidden);
    navigator.clipboard
      ?.writeText(pre?.textContent ?? '')
      .then(() => {
        copy.classList.add('copied');
        clearTimeout((copy as any)._t); // eslint-disable-line @typescript-eslint/no-explicit-any
        (copy as any)._t = setTimeout(() => copy.classList.remove('copied'), 1600); // eslint-disable-line @typescript-eslint/no-explicit-any
      })
      .catch(() => {});
  });

  // ---------- start ----------
  measure();
  const hash = decodeURIComponent(location.hash.slice(1));
  if (items.has(hash)) setActive(hash);
  update();
  on(window, 'scroll', onScroll, { passive: true });
  on(window, 'resize', onResize);
  const ro = new ResizeObserver(onResize);
  ro.observe(document.body);
  cleanups.push(() => ro.disconnect(), () => frame && cancelAnimationFrame(frame));

  return () => cleanups.forEach((fn) => fn());
}
