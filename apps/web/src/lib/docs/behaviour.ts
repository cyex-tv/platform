// cyex docs behaviour (cyex-docs references/behaviour.js), typed: scroll-spy, an index that
// follows the reader both ways, independent column scrolling, the Ctrl/Cmd+K filter and shared
// code-sample tabs. Call it from an $effect once the sections have rendered and return the
// cleanup it gives back.
//
// One change from the skill: the Ctrl/Cmd+K listener runs on window in the capture phase and
// stops there, so on this page the shortcut reaches the filter and not the site's command palette.
//
// Two changes for the site shell: the sticky header is the site's own nav (.nav), so the offset the
// scroll-spy uses is its height + 16 (the same as the site's scroll-padding-top), and the index
// folds into a <details class="nav-toc"> when it is stacked above the text (<= 900px).
//
// Markup it expects (see the skill's starter page):
//   .nav                                        the site's sticky nav
//   aside.docs-nav > .nav-search > input        the filter
//   aside.docs-nav > nav.nav-scroll             the index; its own scroll container
//     .nav-group > .nav-group-title + a.nav-item[href="#id"]
//   section[id]                                 one per index entry, in the same order

export interface DocsOptions {
  headerOffset?: number; // default: the nav's height + 16
  band?: [number, number];
  park?: number;
  langKey?: string;
}

export function initDocs(root: ParentNode = document, opts: DocsOptions = {}): () => void {
  const BAND = opts.band ?? [0.25, 0.6]; // the comfortable zone, as fractions of the rail
  const PARK = opts.park ?? 0.35; // where an item is brought to when it leaves it
  const LANG_KEY = opts.langKey ?? 'cyexDocsLang';

  const header = document.querySelector<HTMLElement>('.nav'); // the site's sticky nav
  const headerOffset = () => opts.headerOffset ?? (header?.getBoundingClientRect().height ?? 64) + 16;
  const aside = root.querySelector<HTMLElement>('.docs-nav')!;
  const nav = root.querySelector<HTMLElement>('.nav-scroll')!;
  const toc = root.querySelector<HTMLDetailsElement>('details.nav-toc');
  const toTop = root.querySelector<HTMLElement>('.to-top');
  const stacked = () => getComputedStyle(aside).position !== 'sticky';
  const search = root.querySelector<HTMLElement>('.nav-search');
  const input = search?.querySelector('input') ?? null;
  const sections = [...root.querySelectorAll<HTMLElement>('section[id]')];
  const items = new Map([...nav.querySelectorAll<HTMLAnchorElement>('a.nav-item')].map((a) => [a.getAttribute('href')!.slice(1), a]));
  const reduced = () => matchMedia('(prefers-reduced-motion: reduce)').matches;
  const cleanups: (() => void)[] = [];
  const on = <E extends Event>(target: EventTarget, type: string, fn: (e: E) => void, options?: AddEventListenerOptions) => {
    target.addEventListener(type, fn as EventListener, options);
    cleanups.push(() => target.removeEventListener(type, fn as EventListener, options));
  };

  // ---------- scroll-spy ----------
  // Offsets are measured once (and again on resize) and compared with scrollY.
  // An IntersectionObserver band thin enough for one section is thin enough for two
  // to straddle it, and then it lights the section you are leaving.
  let tops: { id: string; top: number }[] = [];
  const measure = () => {
    tops = sections.map((s) => ({ id: s.id, top: s.getBoundingClientRect().top + scrollY }));
  };

  let activeId: string | null = null;
  let edge: 'top' | 'end' | null = null; // the page is at one of its ends
  let frame = 0;

  function update() {
    frame = 0;
    if (!tops.length) return;
    const atTop = scrollY <= 2;
    const atEnd = innerHeight + scrollY >= document.documentElement.scrollHeight - 2;
    let current = tops[0];
    if (atEnd) current = tops[tops.length - 1]; // a short last section may never reach the header
    else {
      const y = scrollY + headerOffset();
      // Two pixels of slack: a hash jump parks a section exactly on the boundary.
      for (const t of tops) {
        if (t.top <= y + 2) current = t;
        else break;
      }
    }
    const nextEdge = atTop ? 'top' : atEnd ? 'end' : null;
    if (toTop) toTop.hidden = !(stacked() && scrollY > innerHeight * 2);
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

  // ---------- the rail follows, both ways ----------
  // The lit item (with its group's heading when it leads the group) is kept inside a
  // band around the middle of the rail, so the index travels with the reader going
  // down exactly as it does going up. At the page's top the rail goes to its top; at
  // the page's end, to its end. A rail the reader just scrolled by hand is left alone
  // until the next change of section.
  let railTouchedAt = 0;

  function follow() {
    if (stacked()) return; // stacked layout: the rail isn't beside the text
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
      if (top >= h * BAND[0] && bottom <= h * BAND[1]) return; // already comfortable
      target = nav.scrollTop + top - h * PARK;
    }
    target = Math.max(0, Math.min(max, Math.round(target)));
    if (Math.abs(target - nav.scrollTop) < 2) return;
    nav.scrollTo({ top: target, behavior: reduced() ? 'auto' : 'smooth' });
  }

  // ---------- two columns, two scrolls ----------
  // Whichever column the pointer is over is the one that moves. overscroll-behavior on
  // the rail stops it handing the wheel to the page at its ends; this covers the parts
  // that aren't scroll containers (search box, padding, a filtered-down short list).
  on<WheelEvent>(
    aside,
    'wheel',
    (e) => {
      if (stacked()) return;
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
  on(nav, 'pointerdown', () => {
    railTouchedAt = performance.now();
  });
  on(nav, 'touchmove', () => {
    railTouchedAt = performance.now();
  }, { passive: true });
  // Picking an entry is a request to go there, not a manual scroll of the rail.
  on<MouseEvent>(nav, 'click', (e) => {
    if ((e.target as Element).closest('a.nav-item')) railTouchedAt = 0;
  });

  // ---------- contents disclosure ----------
  // Beside the text the contents are always open; stacked above it they fold away, and picking an
  // entry (or a search that finds some) opens or closes them as needed.
  if (toc) {
    const mq = matchMedia('(min-width: 901px)');
    const sync = () => {
      toc.open = mq.matches;
    };
    sync();
    on(mq, 'change', sync);
    on<MouseEvent>(nav, 'click', (e) => {
      if (stacked() && (e.target as Element).closest('a.nav-item')) toc.open = false;
    });
  }
  if (toTop) on(toTop, 'click', () => scrollTo({ top: 0, behavior: reduced() ? 'auto' : 'smooth' }));

  // ---------- filter ----------
  if (search && input) {
    const kbd = search.querySelector('.nav-kbd');
    if (kbd) kbd.textContent = /Mac|iP(hone|ad|od)/.test(navigator.platform || navigator.userAgent) ? '⌘K' : 'Ctrl K';
    const empty = nav.querySelector<HTMLElement>('.nav-empty');
    const apply = () => {
      const q = input.value.trim().toLowerCase();
      search.classList.toggle('has-value', !!input.value);
      if (toc && q && stacked()) toc.open = true;
      let any = false;
      // Entries match on their own text (label, method, path); a group heading never
      // matches by itself, so "admin" finds admin endpoints by their own names.
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
    const clear = search.querySelector('.input-clear');
    if (clear)
      on(clear, 'click', () => {
        input.value = '';
        apply();
        input.focus();
      });
    on<KeyboardEvent>(
      window,
      'keydown',
      (e) => {
        if ((e.metaKey || e.ctrlKey) && !e.altKey && e.key.toLowerCase() === 'k') {
          e.preventDefault();
          e.stopPropagation(); // the site's command palette listens on window too
          input.focus();
          input.select();
        } else if (e.key === 'Escape' && document.activeElement === input) {
          input.value = '';
          apply();
          input.blur();
        }
      },
      { capture: true },
    );
  }

  // ---------- code samples ----------
  // One language choice for the whole page, remembered: picking Python once shouldn't
  // mean picking it again on the next endpoint.
  const setLang = (lang: string) => {
    for (const tab of root.querySelectorAll<HTMLElement>('.lang-tab')) {
      const sel = tab.dataset.lang === lang;
      tab.setAttribute('aria-selected', String(sel));
      tab.tabIndex = sel ? 0 : -1; // roving: one tab stop per tablist
    }
    for (const pre of root.querySelectorAll<HTMLElement>('pre[data-lang]')) pre.hidden = pre.dataset.lang !== lang;
  };
  let lang = root.querySelector<HTMLElement>('.lang-tab')?.dataset.lang;
  try {
    const saved = localStorage.getItem(LANG_KEY);
    if (saved && root.querySelector(`.lang-tab[data-lang="${CSS.escape(saved)}"]`)) lang = saved;
  } catch {
    /* storage blocked: the first language it is */
  }
  if (lang) setLang(lang);
  const pickLang = (tab: HTMLElement) => {
    setLang(tab.dataset.lang!);
    try {
      localStorage.setItem(LANG_KEY, tab.dataset.lang!);
    } catch {
      /* not remembered, still switched */
    }
  };
  // Arrow keys, Home and End move between a tablist's tabs, switch the shared language and follow with focus.
  on<KeyboardEvent>(document, 'keydown', (e) => {
    const tab = (e.target as Element).closest<HTMLElement>('.lang-tab');
    if (!tab || e.altKey || e.ctrlKey || e.metaKey) return;
    const tabs = [...tab.closest('[role="tablist"]')!.querySelectorAll<HTMLElement>('.lang-tab')];
    const at = tabs.indexOf(tab);
    const next = e.key === 'ArrowRight' ? tabs[(at + 1) % tabs.length] : e.key === 'ArrowLeft' ? tabs[(at - 1 + tabs.length) % tabs.length] : e.key === 'Home' ? tabs[0] : e.key === 'End' ? tabs[tabs.length - 1] : null;
    if (!next) return;
    e.preventDefault();
    pickLang(next);
    next.focus();
  });
  on<MouseEvent>(document, 'click', (e) => {
    const target = e.target as Element;
    const tab = target.closest<HTMLElement>('.lang-tab');
    if (tab) {
      pickLang(tab);
      return;
    }
    const copy = target.closest<HTMLElement & { _t?: ReturnType<typeof setTimeout> }>('.code-copy');
    if (!copy) return;
    const pre = [...copy.closest('.code-block')!.querySelectorAll('pre')].find((p) => !p.hidden);
    if (!pre) return;
    navigator.clipboard
      ?.writeText(pre.textContent ?? '')
      .then(() => {
        copy.classList.add('copied');
        clearTimeout(copy._t);
        copy._t = setTimeout(() => copy.classList.remove('copied'), 1600);
      })
      .catch(() => {});
  });

  // ---------- start ----------
  measure();
  const hash = decodeURIComponent(location.hash.slice(1));
  if (items.has(hash)) setActive(hash); // a deep link lights its entry before any scrolling
  update();
  on(window, 'scroll', onScroll, { passive: true });
  on(window, 'resize', onResize);
  // Sections change height as fonts land and samples fill in; cached offsets go stale.
  // The nav is watched too: its height sets the offset.
  const ro = new ResizeObserver(onResize);
  ro.observe(document.body);
  if (header) ro.observe(header);
  cleanups.push(
    () => ro.disconnect(),
    () => frame && cancelAnimationFrame(frame),
  );

  return () => cleanups.forEach((fn) => fn());
}
