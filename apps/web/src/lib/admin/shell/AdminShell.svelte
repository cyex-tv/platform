<script lang="ts">
  // The admin pages' frame: a full-height rail on the left (flush with the window's edge, a
  // hairline beside it, as the browse page's set rail), the page on the right. The rail sticks
  // under the header while it fits the window and scrolls with the page when it doesn't, so it
  // never has a scroll bar of its own (owner rule). Groups fold; the current row is bold with an
  // accent icon. Under 900px the rail becomes a sticky "Admin" bar whose button opens the same
  // rows in a bottom sheet.
  import type { Snippet } from 'svelte';
  import BottomSheet from '$lib/components/BottomSheet.svelte';
  import Icon from '$lib/components/Icon.svelte';
  import { count as fmt } from '$lib/format';
  import { arrowNav, type RailGroup } from '$lib/shell/rail';
  import { fitSticky } from './fit';
  import '../admin.css';

  type Props = {
    groups: RailGroup[];
    /** The signed-in account, for the "Your profile" foot. */
    me?: string | null;
    children: Snippet;
  };
  let { groups, me = null, children }: Props = $props();

  // Groups the reader folded, by label; a group holding the current row stays open.
  let closed = $state<Record<string, boolean>>({});
  const isOpen = (g: RailGroup) => !g.toggle || g.items.some((i) => i.current) || !closed[g.toggle.label];
  const total = (g: RailGroup) => g.items.reduce((n, i) => n + (typeof i.count === 'number' ? i.count : 0), 0);
  const flat = $derived(groups.flatMap((g) => g.items));
  const current = $derived(flat.find((i) => i.current));
  // The rows are one Tab stop (the current row); the arrow keys move between them.
  const stop = $derived(current?.href ?? flat[0]?.href);
  const has = (c: unknown) => c !== undefined && c !== null && c !== '';
  const shown = (c: number | string | undefined) => (typeof c === 'number' ? fmt(c) : c);

  let sheet = $state(false);
</script>

{#snippet rows(onpick?: () => void)}
  <nav class="nav" aria-label="Admin" use:arrowNav>
    {#each groups as g, gi (gi)}
      <div class="group" class:sub={!!g.toggle}>
        {#if g.toggle}
          {@const open = isOpen(g)}
          <button class="row toggle" type="button" aria-expanded={open} disabled={open && g.items.some((i) => i.current)}
            onclick={() => (closed[g.toggle!.label] = open)}>
            {#if g.toggle.icon}<Icon name={g.toggle.icon} />{/if}
            <span class="lbl">{g.toggle.label}</span>
            {#if !open && total(g) > 0}<span class="n" aria-hidden="true">{fmt(total(g))}</span><span class="sr-only">, {fmt(total(g))}</span>{/if}
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
                  {#if has(i.count)}<span class="n" aria-hidden="true">{shown(i.count)}</span><span class="sr-only">, {shown(i.count)}</span>{/if}
                </a>
              </li>
            {/each}
          </ul>
        {/if}
      </div>
    {/each}
  </nav>
{/snippet}

<div class="wrap adm-shell">
  <aside class="rail" aria-label="Admin sections" use:fitSticky>
    <div class="head"><span class="label">Admin</span><span class="ui-chip">Staff</span></div>
    {@render rows()}
    {#if me}
      <a class="foot" href="/users/{me}"><Icon name="circle-user" /><span class="lbl">Your profile</span><Icon name="chevron-right" size={14} /></a>
    {/if}
  </aside>

  <div class="bar">
    <button class="menu" type="button" aria-haspopup="dialog" aria-expanded={sheet} onclick={() => (sheet = true)}>
      <Icon name="menu" /><span class="k">Admin</span>{#if current}<span class="now">{current.label}</span>{/if}
      {#if current && has(current.count)}<span class="n">{shown(current.count)}</span>{/if}
      <span class="chev"><Icon name="chevron-down" size={14} /></span>
    </button>
  </div>
  <BottomSheet bind:open={sheet} label="Admin">
    <div class="in-sheet">{@render rows(() => (sheet = false))}</div>
  </BottomSheet>

  <div class="main">{@render children()}</div>
</div>

<style>
  .adm-shell { display: grid; grid-template-columns: minmax(0, 1fr); align-items: start; }
  .main { min-width: 0; padding: var(--rail-gap) 0 48px; }
  .rail { display: none; }

  @media (min-width: 900px) {
    .adm-shell { grid-template-columns: 232px minmax(0, 1fr); column-gap: 24px; }
    .bar { display: none; }
    /* Full height, flush with the window's left edge, a hairline on its right: the set rail's shape. */
    .rail { display: flex; flex-direction: column; position: sticky; top: var(--nav-h); align-self: start;
      min-height: calc(100svh - var(--nav-h)); margin-left: calc(-1 * var(--page-pad)); padding: 16px 12px 16px var(--page-pad);
      border-right: 1px solid var(--border); }
    .rail:global([data-tall]) { position: relative; top: auto; }
  }

  .head { display: flex; align-items: center; justify-content: space-between; gap: 8px; min-height: 28px; padding: 0 8px; margin-bottom: 8px; }
  .label { font-size: var(--t-label); font-weight: 600; letter-spacing: .06em; text-transform: uppercase; color: var(--dim); }
  .head .ui-chip { height: 20px; font-size: 11px; }

  .nav { display: grid; gap: 2px; }
  .group { display: grid; gap: 1px; min-width: 0; }
  .group.sub { margin-top: 4px; }
  ul { list-style: none; margin: 0; padding: 0; display: grid; gap: 1px; }
  .row { display: flex; align-items: center; gap: 10px; width: 100%; min-height: 34px; padding: 0 8px; border: 0; border-radius: var(--radius);
    background: none; font: inherit; font-size: var(--t-small); font-weight: 500; color: var(--muted); text-align: left; text-decoration: none;
    transition: background-color var(--fast), color var(--fast); }
  .row :global(svg) { flex: none; color: var(--dim); transition: color var(--fast); }
  .nested .row { padding-left: 34px; }
  .toggle { cursor: pointer; font-size: var(--t-fine); font-weight: 600; }
  .toggle:disabled { cursor: default; }
  .lbl { flex: 1; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
  .n { flex: none; font-size: var(--t-fine); font-weight: 500; color: var(--dim); font-variant-numeric: tabular-nums; }
  .chev { display: inline-grid; flex: none; color: var(--dim); transition: rotate var(--base) var(--ease); }
  .chev.up { rotate: 180deg; }
  /* Selected: bold text and an accent icon (owner rule), never a fill or a bar. */
  .row[aria-current='page'] { color: var(--text); font-weight: 700; }
  .row[aria-current='page'] :global(svg) { color: var(--accent-text); }
  .row:focus-visible, .foot:focus-visible, .menu:focus-visible { outline: 2px solid var(--focus-ring); outline-offset: -2px; }
  .row:active, .foot:active { opacity: .6; transition: none; }

  .foot { display: flex; align-items: center; gap: 10px; min-height: 40px; margin-top: auto; padding: 12px 8px 0; border-top: 1px solid var(--border);
    border-radius: 0; font-size: var(--t-small); font-weight: 500; color: var(--muted); text-decoration: none; }
  .rail .nav { margin-bottom: 16px; }
  .foot :global(svg) { color: var(--dim); flex: none; }

  /* Phones and narrow windows: one sticky bar under the header; the rows open in a sheet. */
  .bar { position: sticky; top: calc(var(--nav-h) + 1px + env(safe-area-inset-top, 0px)); z-index: var(--z-panel);
    margin-inline: calc(-1 * var(--page-pad)); padding: 8px var(--page-pad); background: var(--bg); border-bottom: 1px solid var(--border); }
  .menu { display: flex; align-items: center; gap: 8px; width: 100%; height: 44px; padding: 0 12px; border: 1px solid var(--border-strong);
    border-radius: var(--radius-btn); background: var(--surface); color: var(--text); font: inherit; font-size: var(--t-nav); cursor: pointer; }
  .menu :global(svg) { flex: none; color: var(--dim); }
  .menu .k { font-weight: 600; }
  .menu .now { min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; color: var(--muted); }
  .menu .now::before { content: '·'; margin-right: 8px; color: var(--dim); }
  .menu .chev { margin-left: auto; }
  .in-sheet .row { min-height: 48px; font-size: var(--t-ui); }
  .in-sheet .nested .row { padding-left: 38px; }
  @media (max-height: 499.98px) { .bar { position: static; } }

  @media (hover: hover) and (pointer: fine) {
    .row:not(:disabled):hover { background: var(--row-hover); color: var(--text); }
    .row:hover :global(svg) { color: var(--muted); }
    .row[aria-current='page']:hover :global(svg) { color: var(--accent-text); }
    .foot:hover { color: var(--text); }
  }
  @media (pointer: coarse) { .row { min-height: 44px; } }
  @media (prefers-reduced-motion: reduce) { :global(html:not([data-motion='full'])) .chev { transition: none; } }
  @media (forced-colors: active) { .row[aria-current='page'] { outline: 2px solid Highlight; outline-offset: -2px; } }
</style>
