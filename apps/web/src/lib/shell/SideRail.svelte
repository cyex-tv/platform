<script lang="ts">
  // The one vertical rail (7TV's side bar, cyex styling): context for the page's subject, never a
  // second top bar. 240px on a surface card, sticky under the header while it fits the window;
  // a taller rail scrolls with the page instead, never inside itself (owner: no scroll bar in the
  // category selector). `fill` makes it run the full height of the left side. Groups
  // have a hairline between them (a group is a collapsible row with a count, or a plain caption); rows are icon + label + optional count;
  // the current row is full-contrast and bold with an accent icon (aria-current="page"), no fill or bar. The rows
  // are one Tab stop (the current row); arrow keys move between them. Fewer than two rows: no
  // nav at all, only the head (a profile card needs no one-tab strip).
  //
  // Below 900px it never stacks above the content:
  //   collapse="tabs"  (sections: /me, admin, creator, channel) a horizontal tab strip that sticks
  //                    under the header and scrolls sideways, the current tab kept in view.
  //   collapse="sheet" (filters, docs) a "Filters" or "Menu" button that opens the same content
  //                    in a bottom sheet.
  //
  //   <SideRail label="Your account" groups={[{ caption: 'Library', items }]}>{#snippet head()}…{/snippet}</SideRail>
  import type { Snippet } from 'svelte';
  import BottomSheet from '$lib/components/BottomSheet.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import { count as fmt } from '$lib/format';
  import { arrowNav, type RailGroup, type RailItem } from './rail';

  type Props = {
    /** The rail's name for assistive tech (and the sheet's title). */
    label: string;
    groups?: RailGroup[];
    /** A title or identity block above the groups. */
    head?: Snippet;
    /** Anything below the groups (filter controls, a note). */
    children?: Snippet;
    /** Pinned to the rail's bottom (7TV's "Admin ›"); wide windows only. */
    foot?: Snippet;
    collapse?: 'tabs' | 'sheet';
    /** The sheet button's text (collapse="sheet"): "Filters" or "Menu". */
    sheetLabel?: 'Filters' | 'Menu';
    /** collapse="sheet" only: something kept beside the sheet button in its sticky bar (a search field). */
    bar?: Snippet;
    /** collapse="sheet" only: a number on the sheet button (filters that are on); hidden at 0. */
    sheetCount?: number;
    /** collapse="tabs" only: the phone strip's tabs when they are not just the rows (a page without a row of its own adds its tab). */
    tabs?: RailItem[];
    /** Wide windows: the rail runs the full height of the left side (the window below the header), its foot at the bottom. */
    fill?: boolean;
  };
  let { label, groups = [], head, children, foot, collapse = 'tabs', sheetLabel = 'Menu', bar, sheetCount = 0, tabs, fill = false }: Props = $props();

  // Collapsible groups the reader closed, by their label. A group holding the current row stays open.
  let closed = $state<Record<string, boolean>>({});
  // A closed group still shows its open tickets: the sum of its rows' numbers.
  const total = (g: RailGroup) => g.items.reduce((n, i) => n + (typeof i.count === 'number' ? i.count : 0), 0);
  const isOpen = (g: RailGroup) => !g.toggle || g.items.some((i) => i.current) || !closed[g.toggle.label];

  let narrow = $state(false);
  let sheet = $state(false);
  let strip = $state<HTMLElement>();

  $effect(() => {
    const mq = matchMedia('(max-width: 899.98px)');
    narrow = mq.matches;
    const on = () => {
      narrow = mq.matches;
      if (!narrow) sheet = false;
    };
    mq.addEventListener('change', on);
    return () => mq.removeEventListener('change', on);
  });

  // Edge fades only where there is more to scroll to.
  let fadeStart = $state(false);
  let fadeEnd = $state(false);
  function edges() {
    if (!strip) return;
    fadeStart = strip.scrollLeft > 1;
    fadeEnd = strip.scrollLeft + strip.clientWidth < strip.scrollWidth - 1;
  }
  $effect(() => {
    if (!strip) return;
    const el = strip;
    const ro = new ResizeObserver(edges);
    ro.observe(el);
    el.addEventListener('scroll', edges, { passive: true });
    return () => {
      ro.disconnect();
      el.removeEventListener('scroll', edges);
    };
  });

  // Keep the current tab in view when the strip is narrower than its tabs.
  $effect(() => {
    if (!narrow || !strip) return;
    void groups; void tabs;
    const tab = strip.querySelector<HTMLElement>('[aria-current="page"]');
    if (!tab) return;
    // Scroll only as far as needed, so the first tabs stay readable when the current one already shows.
    const pad = 20;
    const left = tab.offsetLeft - pad;
    const right = tab.offsetLeft + tab.offsetWidth + pad - strip.clientWidth;
    if (left < strip.scrollLeft) strip.scrollLeft = Math.max(0, left);
    else if (right > strip.scrollLeft) strip.scrollLeft = right;
    edges();
  });

  // The desktop rail sticks under the header only while it fits the window. Taller (a short
  // window, every group open), it scrolls with the page, so its last rows are always reachable
  // and it never needs a scroll bar of its own.
  let aside = $state<HTMLElement>();
  let tall = $state(false);
  function fit() {
    if (!aside) return;
    // Where the sticky rail sits (below the header, one gap down) and the gap it keeps at the
    // bottom: from the same custom properties the CSS uses, so the answer doesn't change when
    // the class flips.
    const cs = getComputedStyle(aside);
    const px = (name: string, fallback: number) => parseFloat(cs.getPropertyValue(name)) || fallback;
    const gap = px('--rail-gap', 20);
    const room = innerHeight - px('--nav-h', 56) - 2 * gap;
    tall = aside.offsetHeight > room + 1;
  }
  $effect(() => {
    if (!aside) return;
    const el = aside;
    fit();
    const ro = new ResizeObserver(fit);
    ro.observe(el, { box: 'border-box' });
    // Rows arriving later (counts, a group opening) change the height too.
    const mo = new MutationObserver(() => requestAnimationFrame(fit));
    mo.observe(el, { subtree: true, childList: true, characterData: true });
    addEventListener('resize', fit, { passive: true });
    return () => {
      ro.disconnect();
      mo.disconnect();
      removeEventListener('resize', fit);
    };
  });

  const flat = $derived(groups.flatMap((g) => g.items));
  const strips = $derived(tabs ?? flat);
  const navigable = $derived(flat.length >= 2);
  // The one Tab stop: the current row, else the first.
  const stop = $derived((narrow && collapse === 'tabs' ? strips : flat).find((i) => i.current)?.href ?? flat[0]?.href);
  const current = $derived(flat.find((i) => i.current));
  const shown = (c: RailItem['count']) => (typeof c === 'number' ? fmt(c) : c);
  const has = (c: RailItem['count']) => c != null && c !== '';
</script>

{#snippet rows(onpick?: () => void)}
  {#each groups as g, gi (gi)}
    <div class="group" class:sub={!!g.toggle} class:joined={!g.caption && !g.toggle && gi > 0 && !!groups[gi - 1]?.toggle}>
      {#if g.caption}<p class="caption">{g.caption}</p>{/if}
      {#if g.toggle}
        {@const open = isOpen(g)}
        <button class="row toggle" type="button" aria-expanded={open} onclick={() => (closed[g.toggle!.label] = open)}
          disabled={open && g.items.some((i) => i.current)}>
          {#if g.toggle.icon}<Icon name={g.toggle.icon} />{/if}
          <span class="lbl">{g.toggle.label}</span>
          {#if !open && total(g) > 0}<span class="count" aria-hidden="true">{fmt(total(g))}</span><span class="sr-only">, {fmt(total(g))}</span>{/if}
          <span class="chev" class:up={open}><Icon name="chevron-down" size={14} /></span>
        </button>
      {/if}
      {#if isOpen(g)}
      <ul class:nested={!!g.toggle}>
        {#each g.items as i (i.href)}
          <li>
            <a class="row" href={i.href} aria-current={i.current ? 'page' : undefined} tabindex={i.href === stop ? 0 : -1} onclick={() => onpick?.()}>
              {#if i.icon}<Icon name={i.icon} />{/if}
              <span class="lbl">{i.label}</span>
              {#if has(i.count)}<span class="count" aria-hidden="true">{shown(i.count)}</span><span class="sr-only">, {shown(i.count)}</span>{/if}
            </a>
          </li>
        {/each}
      </ul>
      {/if}
    </div>
  {/each}
{/snippet}

{#snippet body(onpick?: () => void)}
  {#if head}<div class="head" class:alone={!navigable && !children}>{@render head()}</div>{/if}
  {#if navigable}<nav aria-label={label} use:arrowNav>{@render rows(onpick)}</nav>{/if}
  {#if children}<div class="extra">{@render children()}</div>{/if}
  {#if foot && !narrow}<div class="foot">{@render foot()}</div>{/if}
{/snippet}

{#if narrow && collapse === 'tabs'}
  {#if navigable}
  <nav class="strip" class:fade-start={fadeStart} class:fade-end={fadeEnd} aria-label={label} bind:this={strip} use:arrowNav={'horizontal'}>
    {#each strips as i (i.href)}
      <a class="tab" href={i.href} aria-current={i.current ? 'page' : undefined} tabindex={i.href === stop ? 0 : -1}>
        {#if i.icon}<Icon name={i.icon} size={16} />{/if}{i.label}{#if has(i.count)}<span class="count" aria-hidden="true">{shown(i.count)}</span><span class="sr-only">, {shown(i.count)}</span>{/if}
      </a>
    {/each}
  </nav>
  {/if}
{:else if narrow}
  <div class="sheet-bar" class:with-bar={!!bar}>
    {#if bar}<div class="bar-extra">{@render bar()}</div>{/if}
    <button class="btn sheet-btn" type="button" aria-haspopup="dialog" aria-expanded={sheet} onclick={() => (sheet = true)}>
      <Icon name={sheetLabel === 'Filters' ? 'sliders' : 'menu'} />{sheetLabel}
      {#if current}<span class="now">{current.label}</span>{/if}
      {#if sheetCount > 0}<span class="sheet-n">{sheetCount}<span class="sr-only"> on</span></span>{/if}
    </button>
  </div>
  <BottomSheet bind:open={sheet} label={sheetLabel === 'Menu' ? label : sheetLabel}>
    <div class="in-sheet">{@render body(() => (sheet = false))}</div>
  </BottomSheet>
{:else}
  <aside class="rail" class:footed={!!foot} class:fill class:tall aria-label={label} bind:this={aside}>
    {@render body()}
  </aside>
{/if}

<style>
  .rail { position: sticky; top: calc(var(--nav-h) + env(safe-area-inset-top, 0px) + var(--rail-gap)); align-self: start;
    width: var(--rail-w); padding: 12px; background: var(--surface); border: 1px solid var(--border); border-radius: var(--radius-card); }
  /* Taller than the window: it scrolls with the page (sticky would hide its bottom rows). */
  .rail.tall { position: relative; top: auto; }
  /* Before hydration on a narrow screen: no full rail above the content, the strip follows. */
  @media (max-width: 899.98px) { .rail { display: none; } }
  .head { padding: 2px 2px 12px; margin-bottom: 10px; border-bottom: 1px solid var(--border); }
  .head.alone { border-bottom: 0; margin-bottom: 0; }
  .group + .group { margin-top: 10px; padding-top: 10px; border-top: 1px solid var(--border); }
  /* Collapsible groups sit together like rows, without hairlines between them. */
  .group.sub + .group.sub, .group.joined, .group + .group.sub:not(:has(.caption)) { margin-top: 1px; padding-top: 0; border-top: 0; }
  .toggle { width: 100%; border: 0; background: none; font: inherit; font-size: var(--t-nav); text-align: left; cursor: pointer; }
  .toggle:disabled { cursor: default; }
  .chev { display: inline-grid; flex: none; transition: rotate var(--base) var(--ease); }
  .chev.up { rotate: 180deg; }
  .nested { margin-top: 1px; }
  .nested .row { padding-left: 36px; }
  .footed, .fill { display: flex; flex-direction: column; min-height: calc(100dvh - var(--nav-h) - env(safe-area-inset-top, 0px) - 2 * var(--rail-gap)); }
  .foot { margin-top: auto; padding-top: 12px; }
  .foot :global(.rail-foot) { display: flex; align-items: center; gap: 10px; min-height: 40px; padding: 0 10px; margin-top: 10px;
    border-top: 1px solid var(--border); padding-top: 10px; border-radius: 0; font-size: var(--t-nav); font-weight: 500; color: var(--muted); }
  .foot :global(.rail-foot svg) { color: var(--dim); }
  .foot :global(.rail-foot .lbl) { flex: 1; }
  @media (hover: hover) and (pointer: fine) { .foot :global(.rail-foot:hover) { color: var(--text); } }
  .caption { margin: 0 0 4px; padding: 0 10px; font-size: var(--text-label); font-weight: 600; letter-spacing: .06em;
    text-transform: uppercase; color: var(--dim); line-height: 1.8; }
  ul { list-style: none; margin: 0; padding: 0; display: grid; gap: 1px; }
  .row { display: flex; align-items: center; gap: 10px; min-height: 40px; padding: 0 10px; border-radius: var(--radius);
    font-size: var(--t-nav); font-weight: 500; color: var(--muted); transition: background-color var(--fast), color var(--fast); }
  .row :global(svg) { color: var(--dim); transition: color var(--fast); }
  .lbl { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .count { flex: none; font-size: 12px; font-weight: 500; color: var(--dim); font-variant-numeric: tabular-nums; }
  .row[aria-current='page'] { color: var(--text); font-weight: 600; }
  .row[aria-current='page'] :global(svg) { color: var(--accent-text); }
  .row:focus-visible, .tab:focus-visible { outline-offset: -2px; }
  .row:active, .tab:active { opacity: .6; transition: none; }
  .extra { margin-top: 12px; }
  .head + .extra { margin-top: 0; }

  /* Narrow windows and phones: sections become a tab strip that sticks under the header. */
  /* Sticky right under the header (its 56px bar plus its 1px hairline). */
  .strip { position: sticky; top: calc(var(--nav-h) + 1px + env(safe-area-inset-top, 0px)); z-index: var(--z-panel);
    display: flex; gap: 4px; margin-inline: calc(-1 * var(--page-pad)); padding: 8px var(--page-pad);
    overflow-x: auto; scrollbar-width: none; scroll-snap-type: x proximity; scroll-padding-inline: var(--page-pad);
    background: var(--bg); border-bottom: 1px solid var(--border); overscroll-behavior-x: contain;
    --fs: 0px; --fe: 0px;
    -webkit-mask-image: linear-gradient(90deg, transparent, #000 var(--fs), #000 calc(100% - var(--fe)), transparent);
    mask-image: linear-gradient(90deg, transparent, #000 var(--fs), #000 calc(100% - var(--fe)), transparent); }
  .strip.fade-start { --fs: 28px; }
  .strip.fade-end { --fe: 28px; }
  .strip::-webkit-scrollbar { display: none; }
  .tab { flex: none; display: inline-flex; align-items: center; gap: 6px; height: 40px; padding: 0 12px; scroll-snap-align: start;
    border-radius: var(--radius); font-size: var(--t-nav); font-weight: 500; color: var(--muted); white-space: nowrap;
    transition: background-color var(--fast), color var(--fast); }
  .tab :global(svg) { color: var(--dim); }
  .tab[aria-current='page'] { color: var(--text); font-weight: 600; }
  .tab[aria-current='page'] :global(svg) { color: var(--accent-text); }
  .tab[aria-current='page'] .count { color: var(--muted); }
  @media (pointer: coarse) { .tab { height: 44px; } .row { min-height: 44px; } }

  .sheet-bar { position: sticky; top: calc(var(--nav-h) + 1px + env(safe-area-inset-top, 0px)); z-index: var(--z-panel);
    margin-inline: calc(-1 * var(--page-pad)); padding: 8px var(--page-pad); background: var(--bg); border-bottom: 1px solid var(--border); }
  .sheet-n { display: inline-grid; place-items: center; min-width: 18px; height: 18px; padding: 0 5px; border-radius: var(--radius);
    background: var(--accent-dim); color: var(--accent-text); font-size: 12px; font-weight: 600; font-variant-numeric: tabular-nums; }
  .with-bar { display: flex; align-items: center; gap: 8px; }
  .bar-extra { flex: 1; min-width: 0; }
  .with-bar .sheet-btn { flex: none; }
  .with-bar .now { display: none; }
  .sheet-btn { height: 40px; padding: 0 14px; gap: 8px; font-size: var(--t-nav); }
  .now { color: var(--muted); font-weight: 400; }
  .now::before { content: '·'; margin-right: 8px; color: var(--dim); }
  .in-sheet .row { min-height: 48px; font-size: var(--t-ui); }
  /* Short landscape phones: the header alone stays pinned, the strip scrolls with the page. */
  @media (max-height: 499.98px) { .strip, .sheet-bar { position: static; } }

  @media (hover: hover) and (pointer: fine) {
    .row:hover, .tab:hover { background: var(--row-hover); color: var(--text); }
    .row:hover :global(svg) { color: var(--muted); }
    .row[aria-current='page']:hover, .tab[aria-current='page']:hover { color: var(--text); }
  }
  @media (prefers-reduced-motion: reduce) { .chev { transition: none; } }
  @media (forced-colors: active) {
    .row[aria-current='page'], .tab[aria-current='page'] { outline: 2px solid Highlight; outline-offset: -2px; }
  }
</style>
