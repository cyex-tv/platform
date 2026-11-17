<script lang="ts">
  // The one vertical rail (7TV's side bar, cyex styling): context for the page's subject, never a
  // second top bar. 240px on a surface card, sticky under the header with its own scroll. Groups
  // have a quiet caption and a hairline between them; rows are icon + label + optional count;
  // the current row has the accent-dim fill and a 2px accent rail (aria-current="page"). Arrow
  // keys move between rows.
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
    collapse?: 'tabs' | 'sheet';
    /** The sheet button's text (collapse="sheet"): "Filters" or "Menu". */
    sheetLabel?: 'Filters' | 'Menu';
    /** collapse="sheet" only: something kept beside the sheet button in its sticky bar (a search field). */
    bar?: Snippet;
    /** collapse="sheet" only: a number on the sheet button (filters that are on); hidden at 0. */
    sheetCount?: number;
  };
  let { label, groups = [], head, children, collapse = 'tabs', sheetLabel = 'Menu', bar, sheetCount = 0 }: Props = $props();

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

  // Keep the current tab in view when the strip is narrower than its tabs.
  $effect(() => {
    if (!narrow || !strip) return;
    void groups;
    const tab = strip.querySelector<HTMLElement>('[aria-current="page"]');
    if (!tab) return;
    // Scroll only as far as needed, so the first tabs stay readable when the current one already shows.
    const pad = 20;
    const left = tab.offsetLeft - pad;
    const right = tab.offsetLeft + tab.offsetWidth + pad - strip.clientWidth;
    if (left < strip.scrollLeft) strip.scrollLeft = Math.max(0, left);
    else if (right > strip.scrollLeft) strip.scrollLeft = right;
  });

  const flat = $derived(groups.flatMap((g) => g.items));
  const current = $derived(flat.find((i) => i.current));
  const shown = (c: RailItem['count']) => (typeof c === 'number' ? fmt(c) : c);
  const has = (c: RailItem['count']) => c != null && c !== '';
</script>

{#snippet rows(onpick?: () => void)}
  {#each groups as g, gi (gi)}
    <div class="group">
      {#if g.caption}<p class="caption">{g.caption}</p>{/if}
      <ul>
        {#each g.items as i (i.href)}
          <li>
            <a class="row" href={i.href} aria-current={i.current ? 'page' : undefined} onclick={() => onpick?.()}>
              {#if i.icon}<Icon name={i.icon} />{/if}
              <span class="lbl">{i.label}</span>
              {#if has(i.count)}<span class="count">{shown(i.count)}</span>{/if}
            </a>
          </li>
        {/each}
      </ul>
    </div>
  {/each}
{/snippet}

{#snippet body(onpick?: () => void)}
  {#if head}<div class="head">{@render head()}</div>{/if}
  {#if groups.length}<nav aria-label={label} use:arrowNav>{@render rows(onpick)}</nav>{/if}
  {#if children}<div class="extra">{@render children()}</div>{/if}
{/snippet}

{#if narrow && collapse === 'tabs'}
  <nav class="strip" aria-label={label} bind:this={strip} use:arrowNav={'horizontal'}>
    {#each flat as i (i.href)}
      <a class="tab" href={i.href} aria-current={i.current ? 'page' : undefined}>
        {i.label}{#if has(i.count)}<span class="count">{shown(i.count)}</span>{/if}
      </a>
    {/each}
  </nav>
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
  <aside class="rail">{@render body()}</aside>
{/if}

<style>
  .rail { position: sticky; top: calc(var(--nav-h) + env(safe-area-inset-top, 0px) + var(--rail-gap)); align-self: start;
    width: var(--rail-w); max-height: calc(100dvh - var(--nav-h) - env(safe-area-inset-top, 0px) - 2 * var(--rail-gap));
    overflow-y: auto; overscroll-behavior: contain; scrollbar-width: thin; scrollbar-color: var(--border-strong) transparent;
    padding: 12px; background: var(--surface); border: 1px solid var(--border); border-radius: var(--radius-card); }
  /* Before hydration on a narrow screen: no full rail above the content, the strip follows. */
  @media (max-width: 899.98px) { .rail { display: none; } }
  .head { padding: 2px 2px 12px; margin-bottom: 10px; border-bottom: 1px solid var(--border); }
  .group + .group { margin-top: 10px; padding-top: 10px; border-top: 1px solid var(--border); }
  .caption { margin: 0 0 4px; padding: 0 10px; font-size: var(--text-label); font-weight: 600; letter-spacing: .06em;
    text-transform: uppercase; color: var(--dim); line-height: 1.8; }
  ul { list-style: none; margin: 0; padding: 0; display: grid; gap: 1px; }
  .row { display: flex; align-items: center; gap: 10px; min-height: 36px; padding: 0 10px; border-radius: var(--radius);
    font-size: var(--t-nav); font-weight: 500; color: var(--muted); transition: background-color var(--fast), color var(--fast); }
  .row :global(svg) { color: var(--dim); transition: color var(--fast); }
  .lbl { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .count { flex: none; font-size: 12px; font-weight: 500; color: var(--dim); font-variant-numeric: tabular-nums; }
  .row[aria-current='page'] { background: var(--accent-dim); color: var(--accent-text); box-shadow: inset 2px 0 0 var(--accent); }
  .row[aria-current='page'] :global(svg), .row[aria-current='page'] .count { color: currentColor; }
  .row:focus-visible, .tab:focus-visible { outline-offset: -2px; }
  .row:active, .tab:active { opacity: .6; transition: none; }
  .extra { margin-top: 12px; }
  .head + .extra { margin-top: 0; }

  /* Narrow windows and phones: sections become a tab strip that sticks under the header. */
  .strip { position: sticky; top: calc(var(--nav-h) + env(safe-area-inset-top, 0px)); z-index: var(--z-panel);
    display: flex; gap: 4px; margin-inline: calc(-1 * var(--page-pad)); padding: 8px var(--page-pad);
    overflow-x: auto; scrollbar-width: none; scroll-snap-type: x proximity; scroll-padding-inline: var(--page-pad);
    background: var(--bg); border-bottom: 1px solid var(--border); overscroll-behavior-x: contain;
    -webkit-mask-image: linear-gradient(90deg, #000 calc(100% - 28px), transparent);
    mask-image: linear-gradient(90deg, #000 calc(100% - 28px), transparent); }
  .strip::-webkit-scrollbar { display: none; }
  .tab { flex: none; display: inline-flex; align-items: center; gap: 6px; height: 36px; padding: 0 12px; scroll-snap-align: start;
    border-radius: var(--radius); font-size: var(--t-nav); font-weight: 500; color: var(--muted); white-space: nowrap;
    transition: background-color var(--fast), color var(--fast); }
  .tab[aria-current='page'] { background: var(--accent-dim); color: var(--accent-text); box-shadow: inset 0 -2px 0 var(--accent); }
  .tab[aria-current='page'] .count { color: currentColor; }
  @media (pointer: coarse) { .tab { height: 44px; } .row { min-height: 44px; } }

  .sheet-bar { position: sticky; top: calc(var(--nav-h) + env(safe-area-inset-top, 0px)); z-index: var(--z-panel);
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

  @media (hover: hover) and (pointer: fine) {
    .row:hover, .tab:hover { background: var(--row-hover); color: var(--text); }
    .row:hover :global(svg) { color: var(--muted); }
    .row[aria-current='page']:hover, .tab[aria-current='page']:hover { background: var(--accent-dim); color: var(--accent-text); }
  }
  @media (forced-colors: active) {
    .row[aria-current='page'], .tab[aria-current='page'] { outline: 2px solid Highlight; outline-offset: -2px; }
  }
</style>
