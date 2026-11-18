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
// Groups: every [data-group] element (an index group and its body group share an id) opens and closes
// together. They start closed, except the notes; what the reader toggles is kept in sessionStorage. A
// link to a section in a closed group opens the group first (without animating, so the jump lands
// right), the filter shows matching index groups open, and the scroll-spy only counts sections whose
// group is open. Measured offsets are redone as groups change height.
//
// Markup it expects (see the skill's starter page):
//   .nav                                        the site's sticky nav
//   aside.docs-nav > .nav-search > input        the filter
//   aside.docs-nav > nav.nav-scroll             the index; its own scroll container
//     .nav-group > .nav-group-title + a.nav-item[href="#id"]
//   section[id]                                 one per index entry, in the same order
//   [data-group=id][data-open]                  a group, in the index and in the body; its toggle is
//                                               button[data-group-toggle=id][aria-expanded]
//   button[data-group-all]                      "Expand all" / "Collapse all"

import { GROUP_KEY, allOpen, isOpen, parseGroupState, withAll, withGroup, type GroupState } from './groups';

export interface DocsOptions {
  headerOffset?: number; // default: the nav's height + 16
  band?: [number, number];
  park?: number;
  langKey?: string;
  /** false: the page filters its own index and body (/help); the filter block is left out. */
  filter?: boolean;
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
  const docs = aside.closest<HTMLElement>('.docs') ?? aside;
  const allBtn = root.querySelector<HTMLButtonElement>('[data-group-all]');
  const tools = root.querySelector<HTMLElement>('.nav-tools');
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
  // Only sections whose group is open count: a closed group's sections have no height of their own.
  let tops: { id: string; top: number }[] = [];
  const measure = () => {
    tops = sections.filter((s) => s.closest('[data-group]')?.getAttribute('data-open') !== 'false').map((s) => ({ id: s.id, top: s.getBoundingClientRect().top + scrollY }));
  };
  let dirty = false;

  let activeId: string | null = null;
  let edge: 'top' | 'end' | null = null; // the page is at one of its ends
  let frame = 0;

  function update() {
    frame = 0;
    if (dirty) {
      dirty = false;
      measure();
    }
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
  // Layout changed (resize, fonts, a group opening): measure again, once per frame.
  const onResize = () => {
    dirty = true;
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

  // ---------- groups ----------
  const toggles = [...root.querySelectorAll<HTMLButtonElement>('[data-group-toggle]')];
  const groupIds = [...new Set(toggles.map((b) => b.dataset.groupToggle!))];
  let state: GroupState = {};
  try {
    state = parseGroupState(sessionStorage.getItem(GROUP_KEY));
  } catch {
    /* storage blocked: the defaults */
  }
  const filtering = () => aside.classList.contains('filtering');

  function paint() {
    for (const el of root.querySelectorAll<HTMLElement>('[data-group]')) el.dataset.open = String(isOpen(state, el.dataset.group!));
    const f = filtering();
    for (const b of toggles) {
      const inIndex = !!b.closest('.docs-nav');
      b.setAttribute('aria-expanded', String(isOpen(state, b.dataset.groupToggle!) || (f && inIndex)));
      if (inIndex && f) b.setAttribute('aria-disabled', 'true');
      else b.removeAttribute('aria-disabled');
    }
    if (allBtn) allBtn.textContent = allOpen(state, groupIds) ? 'Collapse all' : 'Expand all';
  }
  // Changes that must not animate (links, expand all, the first paint) switch the duration to 0 for
  // a couple of frames, so the layout is final before anything scrolls.
  let settle = 0;
  function commit(next: GroupState, instant: boolean) {
    state = next;
    try {
      sessionStorage.setItem(GROUP_KEY, JSON.stringify(state));
    } catch {
      /* not remembered, still changed */
    }
    if (instant) {
      docs.classList.add('no-anim');
      cancelAnimationFrame(settle);
      settle = requestAnimationFrame(() => (settle = requestAnimationFrame(() => docs.classList.remove('no-anim'))));
    }
    paint();
    if (instant) {
      measure();
      dirty = false;
    }
    onScroll();
  }
  cleanups.push(() => cancelAnimationFrame(settle));

  /** Opens the group that holds the element with this id. True when that changed anything. */
  function reveal(id: string): boolean {
    const group = document.getElementById(id)?.closest<HTMLElement>('[data-group]')?.dataset.group;
    if (!group || isOpen(state, group)) return false;
    commit(withGroup(state, group, true), true);
    return true;
  }

  for (const b of toggles)
    on(b, 'click', () => {
      const id = b.dataset.groupToggle!;
      if (b.closest('.docs-nav')) {
        if (filtering()) return; // the filter shows its matches open; nothing to toggle
        railTouchedAt = performance.now();
      }
      commit(withGroup(state, id, !isOpen(state, id)), false);
    });
  if (allBtn)
    on(allBtn, 'click', () => {
      railTouchedAt = performance.now();
      commit(withAll(groupIds, !allOpen(state, groupIds)), true);
    });
  // Any in-page link (index entries, "see the events subscription", type names) lands in an open group.
  on<MouseEvent>(root, 'click', (e) => {
    const a = (e.target as Element).closest<HTMLAnchorElement>('a[href^="#"]');
    if (a && a.getAttribute('href')!.length > 1) reveal(decodeURIComponent(a.getAttribute('href')!.slice(1)));
  });
  // Back/forward and typed hashes: the browser may already have jumped to a hidden target.
  const onHash = () => {
    const id = decodeURIComponent(location.hash.slice(1));
    if (id && reveal(id)) requestAnimationFrame(() => document.getElementById(id)?.scrollIntoView({ behavior: 'instant' }));
  };
  on(window, 'hashchange', onHash);
  on(window, 'popstate', onHash);

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
    const mq = matchMedia('(min-width: 900px)');
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
  if (search && input && opts.filter !== false) {
    const kbd = search.querySelector('.nav-kbd');
    if (kbd) kbd.textContent = /Mac|iP(hone|ad|od)/.test(navigator.platform || navigator.userAgent) ? '⌘K' : 'Ctrl K';
    const empty = nav.querySelector<HTMLElement>('.nav-empty');
    const status = nav.querySelector<HTMLElement>('[data-filter-status]');
    const apply = () => {
      const q = input.value.trim().toLowerCase();
      search.classList.toggle('has-value', !!input.value);
      aside.classList.toggle('filtering', !!q); // matching groups show open while a filter is on
      if (tools) tools.hidden = !!q;
      paint();
      if (toc && q && stacked()) toc.open = true;
      let any = false;
      let total = 0;
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
        const count = group.querySelector<HTMLElement>('.group-count');
        if (count) count.textContent = q ? String(shown) : (count.dataset.total ?? count.textContent);
        if (shown) any = true;
        total += shown;
      }
      // One short line for screen readers: how many entries match (nothing while no filter is on).
      if (status) status.textContent = q ? (total ? `${total} ${total === 1 ? 'entry matches' : 'entries match'}` : 'Nothing matches') : '';
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
  const hash = decodeURIComponent(location.hash.slice(1));
  // The saved groups, plus the one a deep link points into, before anything is measured.
  const hashGroup = hash ? document.getElementById(hash)?.closest<HTMLElement>('[data-group]')?.dataset.group : undefined;
  commit(hashGroup && !isOpen(state, hashGroup) ? withGroup(state, hashGroup, true) : state, true);
  if (hashGroup) {
    // The browser jumped to the fragment before the group was open: go there again.
    const jump = () => document.getElementById(hash)?.scrollIntoView({ behavior: 'instant' });
    jump();
    const t = setTimeout(jump, 120);
    cleanups.push(() => clearTimeout(t));
  }
  measure();
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
